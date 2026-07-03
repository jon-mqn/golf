use golf_engine::score::score_hand;
use golf_engine::{
    Action, Event, MatchConfig, MatchState, Phase, RuleError, SeatConfig, SeatKind, TurnState,
};

fn config(players: usize, holes: u8) -> MatchConfig {
    MatchConfig {
        seats: (0..players)
            .map(|i| SeatConfig {
                name: format!("P{i}"),
                kind: SeatKind::Human,
            })
            .collect(),
        holes,
    }
}

#[test]
fn deal_shape() {
    let m = MatchState::new(config(4, 9), 7).unwrap();
    assert_eq!(m.hole.deck.len(), 52 - 4 * 6 - 1);
    assert_eq!(m.hole.discard.len(), 1);
    assert_eq!(m.hole.players.len(), 4);
    for p in &m.hole.players {
        assert_eq!(p.grid.len(), 6);
        assert!(p.grid.iter().all(|s| !s.face_up));
        // Bottom row (slots 3–5) is privately known, top row is not.
        assert!(p.grid[..3].iter().all(|s| !s.known_to_owner));
        assert!(p.grid[3..].iter().all(|s| s.known_to_owner));
    }
    assert_eq!(m.hole.current, 0);
    assert_eq!(m.hole.turn, TurnState::AwaitDraw);
}

#[test]
fn player_count_limits() {
    assert!(MatchState::new(config(1, 9), 0).is_err());
    assert!(MatchState::new(config(5, 9), 0).is_err());
    assert!(MatchState::new(config(2, 0), 0).is_err());
    assert!(MatchState::new(config(2, 9), 0).is_ok());
}

#[test]
fn scripted_turn_draw_flip_swap() {
    let mut m = MatchState::new(config(2, 1), 42).unwrap();
    let top_of_deck = *m.hole.deck.last().unwrap();
    let events = m.apply(0, Action::DrawFromDeck).unwrap();
    assert_eq!(
        events,
        vec![Event::DrewFromDeck {
            seat: 0,
            card: top_of_deck
        }]
    );

    let flipped_card = m.hole.players[0].grid[0].card;
    let events = m.apply(0, Action::Flip { slot: 0 }).unwrap();
    assert_eq!(
        events,
        vec![Event::Flipped {
            seat: 0,
            slot: 0,
            card: flipped_card
        }]
    );
    assert!(m.hole.players[0].grid[0].face_up);

    let events = m.apply(0, Action::Swap).unwrap();
    assert_eq!(
        events,
        vec![
            Event::Swapped {
                seat: 0,
                slot: 0,
                placed: top_of_deck,
                discarded: flipped_card
            },
            Event::TurnStarted { seat: 1 }
        ]
    );
    // The drawn card is now frozen in slot 0; the old card tops the discard.
    assert_eq!(m.hole.players[0].grid[0].card, top_of_deck);
    assert_eq!(*m.hole.discard.last().unwrap(), flipped_card);
    assert_eq!(m.hole.current, 1);
}

#[test]
fn take_discard_then_keep_returns_it() {
    let mut m = MatchState::new(config(2, 1), 42).unwrap();
    let top = *m.hole.discard.last().unwrap();
    m.apply(0, Action::TakeDiscard).unwrap();
    assert!(m.hole.discard.is_empty());
    m.apply(0, Action::Flip { slot: 4 }).unwrap();
    m.apply(0, Action::Keep).unwrap();
    // The "free flip": the same card is back on the discard.
    assert_eq!(*m.hole.discard.last().unwrap(), top);
    assert!(m.hole.players[0].grid[4].face_up);
}

#[test]
fn illegal_actions_are_rejected() {
    let mut m = MatchState::new(config(2, 2), 1).unwrap();
    // Wrong actor.
    assert_eq!(
        m.apply(1, Action::DrawFromDeck),
        Err(RuleError::NotYourTurn)
    );
    // Nonexistent seat.
    assert_eq!(m.apply(7, Action::DrawFromDeck), Err(RuleError::BadSeat));
    // Wrong phase within the turn.
    assert_eq!(
        m.apply(0, Action::Flip { slot: 0 }),
        Err(RuleError::WrongPhase)
    );
    assert_eq!(m.apply(0, Action::Swap), Err(RuleError::WrongPhase));
    assert_eq!(m.apply(0, Action::NextHole), Err(RuleError::WrongPhase));

    m.apply(0, Action::DrawFromDeck).unwrap();
    assert_eq!(m.apply(0, Action::DrawFromDeck), Err(RuleError::WrongPhase));
    assert_eq!(
        m.apply(0, Action::Flip { slot: 9 }),
        Err(RuleError::BadSlot)
    );
    m.apply(0, Action::Flip { slot: 2 }).unwrap();
    assert_eq!(
        m.apply(0, Action::Flip { slot: 3 }),
        Err(RuleError::WrongPhase)
    );
    m.apply(0, Action::Keep).unwrap();

    // Seat 1: flipping seat 0's now-frozen slot 2 is fine for seat 1's own
    // grid, but flipping a frozen slot of their own is rejected.
    m.apply(1, Action::DrawFromDeck).unwrap();
    m.apply(1, Action::Flip { slot: 1 }).unwrap();
    m.apply(1, Action::Keep).unwrap();
    m.apply(0, Action::DrawFromDeck).unwrap();
    assert_eq!(
        m.apply(0, Action::Flip { slot: 2 }),
        Err(RuleError::SlotFrozen)
    );
}

/// Play a full hole with fixed simple choices and check the bookkeeping.
#[test]
fn full_hole_ends_after_six_turns_each() {
    let mut m = MatchState::new(config(3, 2), 99).unwrap();
    let mut turns = [0u32; 3];
    while m.phase == Phase::Playing {
        let seat = m.seat_to_act().unwrap();
        turns[seat as usize] += 1;
        m.apply(seat, Action::DrawFromDeck).unwrap();
        let slot = m.hole.players[seat as usize]
            .grid
            .iter()
            .position(|s| !s.face_up)
            .unwrap() as u8;
        m.apply(seat, Action::Flip { slot }).unwrap();
        m.apply(seat, Action::Keep).unwrap();
    }
    assert_eq!(turns, [6, 6, 6]);
    let Phase::HoleComplete { scores } = &m.phase else {
        panic!("expected HoleComplete");
    };
    // Keeping every dealt card means scores equal the dealt hands' values.
    for (seat, score) in scores.iter().enumerate() {
        assert_eq!(*score, score_hand(&m.hole.players[seat].cards()));
    }
    assert_eq!(m.totals, *scores);
    assert_eq!(m.score_history, vec![scores.clone()]);

    // Next hole: seat 1 starts (rotation), fresh deal, phase Playing.
    m.apply(0, Action::NextHole).unwrap();
    assert_eq!(m.phase, Phase::Playing);
    assert_eq!(m.hole_number, 2);
    assert_eq!(m.hole.current, 1);
    assert!(m.hole.players.iter().all(|p| !p.all_face_up()));
}

#[test]
fn match_ends_with_winners_after_last_hole() {
    let mut m = MatchState::new(config(2, 1), 5).unwrap();
    while m.phase == Phase::Playing {
        let seat = m.seat_to_act().unwrap();
        m.apply(seat, Action::DrawFromDeck).unwrap();
        let slot = m.hole.players[seat as usize]
            .grid
            .iter()
            .position(|s| !s.face_up)
            .unwrap() as u8;
        m.apply(seat, Action::Flip { slot }).unwrap();
        m.apply(seat, Action::Swap).unwrap();
    }
    let events = m.apply(0, Action::NextHole).unwrap();
    let Phase::MatchComplete { winners } = &m.phase else {
        panic!("expected MatchComplete");
    };
    let best = *m.totals.iter().min().unwrap();
    for &w in winners {
        assert_eq!(m.totals[w as usize], best);
    }
    assert!(matches!(events[0], Event::MatchEnded { .. }));
    // Nothing further is accepted.
    assert_eq!(m.apply(0, Action::NextHole), Err(RuleError::MatchOver));
    assert_eq!(m.apply(0, Action::DrawFromDeck), Err(RuleError::MatchOver));
}
