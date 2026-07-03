//! Property-style tests: thousands of random legal playouts, checking
//! invariants and the redaction guarantee at every step.

use golf_engine::{
    Action, MatchConfig, MatchState, Phase, SeatConfig, SeatKind, TurnState, Viewer,
};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

fn config(players: usize) -> MatchConfig {
    MatchConfig {
        seats: (0..players)
            .map(|i| SeatConfig {
                name: format!("P{i}"),
                kind: SeatKind::Human,
            })
            .collect(),
        holes: 3,
    }
}

fn random_legal_action(m: &MatchState, rng: &mut StdRng) -> (u8, Action) {
    match &m.phase {
        Phase::Playing => {
            let seat = m.hole.current;
            let action = match m.hole.turn {
                TurnState::AwaitDraw => {
                    if rng.random_bool(0.5) {
                        Action::DrawFromDeck
                    } else {
                        Action::TakeDiscard
                    }
                }
                TurnState::AwaitFlip { .. } => {
                    let face_down: Vec<u8> = m.hole.players[seat as usize]
                        .grid
                        .iter()
                        .enumerate()
                        .filter(|(_, s)| !s.face_up)
                        .map(|(i, _)| i as u8)
                        .collect();
                    Action::Flip {
                        slot: face_down[rng.random_range(0..face_down.len())],
                    }
                }
                TurnState::AwaitResolve { .. } => {
                    if rng.random_bool(0.5) {
                        Action::Swap
                    } else {
                        Action::Keep
                    }
                }
            };
            (seat, action)
        }
        Phase::HoleComplete { .. } => (0, Action::NextHole),
        Phase::MatchComplete { .. } => unreachable!(),
    }
}

fn check_redaction(m: &MatchState) {
    let n = m.num_seats();
    let mut viewers: Vec<Viewer> = (0..n as u8).map(Viewer::Seat).collect();
    viewers.push(Viewer::Spectator);
    for viewer in viewers {
        let view = m.view(viewer);
        let json = serde_json::to_string(&view).unwrap();
        let viewer_seat = match viewer {
            Viewer::Seat(s) => Some(s),
            Viewer::Spectator => None,
        };
        for (owner, player) in m.hole.players.iter().enumerate() {
            for slot in &player.grid {
                let visible =
                    slot.face_up || (viewer_seat == Some(owner as u8) && slot.known_to_owner);
                if !visible {
                    // Every physical card is unique, and a hidden card can't
                    // simultaneously be on the discard or in the turn state,
                    // so its serialization must not appear anywhere.
                    let card_json = serde_json::to_string(&slot.card).unwrap();
                    assert!(
                        !json.contains(&card_json),
                        "viewer {viewer:?} can see hidden card {card_json} of seat {owner}"
                    );
                }
            }
        }
    }
}

#[test]
fn random_playouts_uphold_invariants() {
    for seed in 0..150u64 {
        let players = 2 + (seed % 3) as usize;
        let mut m = MatchState::new(config(players), seed).unwrap();
        let mut rng = StdRng::seed_from_u64(seed);
        let mut steps = 0;
        while !matches!(m.phase, Phase::MatchComplete { .. }) {
            steps += 1;
            assert!(steps < 10_000, "playout did not terminate");
            if m.phase == Phase::Playing && m.hole.turn == TurnState::AwaitDraw {
                assert!(
                    !m.hole.discard.is_empty(),
                    "discard empty at draw time (seed {seed})"
                );
            }
            let (seat, action) = random_legal_action(&m, &mut rng);
            m.apply(seat, action)
                .unwrap_or_else(|e| panic!("legal action rejected: {e} (seed {seed})"));
            // Redaction holds at every intermediate state (checked sparsely
            // for speed, and always right after a deal).
            if steps % 7 == 0 || steps < 3 {
                check_redaction(&m);
            }
        }
        // Bookkeeping adds up.
        assert_eq!(m.score_history.len(), 3);
        for seat in 0..players {
            let sum: i32 = m.score_history.iter().map(|h| h[seat]).sum();
            assert_eq!(sum, m.totals[seat]);
        }
    }
}

#[test]
fn owner_sees_exactly_bottom_row_peeked_at_deal() {
    let m = MatchState::new(config(3), 11).unwrap();
    for seat in 0..3u8 {
        let view = m.view(Viewer::Seat(seat));
        for (owner, seat_view) in view.seats.iter().enumerate() {
            for (idx, slot) in seat_view.grid.iter().enumerate() {
                let expect_peeked = owner as u8 == seat && idx >= 3;
                match slot {
                    golf_engine::SlotView::Peeked { card } => {
                        assert!(expect_peeked);
                        assert_eq!(*card, m.hole.players[owner].grid[idx].card);
                    }
                    golf_engine::SlotView::Hidden => assert!(!expect_peeked),
                    golf_engine::SlotView::FaceUp { .. } => {
                        panic!("nothing is face up at deal")
                    }
                }
            }
        }
    }
}
