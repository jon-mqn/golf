use crate::action::{Action, RuleError};
use crate::event::Event;
use crate::score::score_hand;
use crate::state::{deal_hole, DrawSource, MatchState, Phase, Seat, TurnState};

impl MatchState {
    /// Validate and apply an action for `actor`, returning the public events
    /// it produced. This is the engine's single mutation entry point.
    pub fn apply(&mut self, actor: Seat, action: Action) -> Result<Vec<Event>, RuleError> {
        if actor as usize >= self.num_seats() {
            return Err(RuleError::BadSeat);
        }
        match self.phase {
            Phase::MatchComplete { .. } => Err(RuleError::MatchOver),
            Phase::HoleComplete { .. } => match action {
                Action::NextHole => Ok(self.next_hole()),
                _ => Err(RuleError::WrongPhase),
            },
            Phase::Playing => {
                if actor != self.hole.current {
                    return Err(RuleError::NotYourTurn);
                }
                self.apply_turn_action(action)
            }
        }
    }

    fn apply_turn_action(&mut self, action: Action) -> Result<Vec<Event>, RuleError> {
        let seat = self.hole.current;
        match (self.hole.turn, action) {
            (TurnState::AwaitDraw, Action::DrawFromDeck) => {
                // 4 players consume at most 25 (deal) + 24 (draws) of 52 cards.
                let card = self
                    .hole
                    .deck
                    .pop()
                    .expect("deck cannot run out with at most 4 players");
                self.hole.turn = TurnState::AwaitFlip {
                    drawn: card,
                    source: DrawSource::Deck,
                };
                Ok(vec![Event::DrewFromDeck { seat, card }])
            }
            (TurnState::AwaitDraw, Action::TakeDiscard) => {
                // Seeded at deal and every turn ends with a discard.
                let card = self
                    .hole
                    .discard
                    .pop()
                    .expect("discard is never empty at draw time");
                self.hole.turn = TurnState::AwaitFlip {
                    drawn: card,
                    source: DrawSource::Discard,
                };
                Ok(vec![Event::TookDiscard { seat, card }])
            }
            (TurnState::AwaitFlip { drawn, source }, Action::Flip { slot }) => {
                let slot_ref = self.hole.players[seat as usize]
                    .grid
                    .get_mut(slot as usize)
                    .ok_or(RuleError::BadSlot)?;
                if slot_ref.face_up {
                    return Err(RuleError::SlotFrozen);
                }
                slot_ref.face_up = true;
                let card = slot_ref.card;
                self.hole.turn = TurnState::AwaitResolve {
                    drawn,
                    source,
                    flipped: slot,
                };
                Ok(vec![Event::Flipped { seat, slot, card }])
            }
            (TurnState::AwaitResolve { drawn, flipped, .. }, Action::Swap) => {
                let slot_ref = &mut self.hole.players[seat as usize].grid[flipped as usize];
                let discarded = slot_ref.card;
                slot_ref.card = drawn;
                self.hole.discard.push(discarded);
                let mut events = vec![Event::Swapped {
                    seat,
                    slot: flipped,
                    placed: drawn,
                    discarded,
                }];
                events.extend(self.finish_turn());
                Ok(events)
            }
            (TurnState::AwaitResolve { drawn, flipped, .. }, Action::Keep) => {
                self.hole.discard.push(drawn);
                let mut events = vec![Event::Kept {
                    seat,
                    slot: flipped,
                    discarded: drawn,
                }];
                events.extend(self.finish_turn());
                Ok(events)
            }
            _ => Err(RuleError::WrongPhase),
        }
    }

    fn finish_turn(&mut self) -> Vec<Event> {
        self.hole.turn = TurnState::AwaitDraw;
        if self.hole.players.iter().all(|p| p.all_face_up()) {
            let scores: Vec<i32> = self
                .hole
                .players
                .iter()
                .map(|p| score_hand(&p.cards()))
                .collect();
            for (total, score) in self.totals.iter_mut().zip(&scores) {
                *total += score;
            }
            self.score_history.push(scores.clone());
            let event = Event::HoleEnded {
                hole: self.hole_number,
                scores: scores.clone(),
                totals: self.totals.clone(),
            };
            self.phase = Phase::HoleComplete { scores };
            vec![event]
        } else {
            self.hole.current = (self.hole.current + 1) % self.num_seats() as Seat;
            vec![Event::TurnStarted {
                seat: self.hole.current,
            }]
        }
    }

    fn next_hole(&mut self) -> Vec<Event> {
        if self.hole_number >= self.config.holes {
            let best = *self.totals.iter().min().expect("at least two seats");
            let winners: Vec<Seat> = self
                .totals
                .iter()
                .enumerate()
                .filter(|(_, &total)| total == best)
                .map(|(seat, _)| seat as Seat)
                .collect();
            self.phase = Phase::MatchComplete {
                winners: winners.clone(),
            };
            vec![Event::MatchEnded {
                totals: self.totals.clone(),
                winners,
            }]
        } else {
            self.hole_number += 1;
            let num_seats = self.num_seats();
            let starting = ((self.hole_number as usize - 1) % num_seats) as Seat;
            self.hole = deal_hole(&mut self.rng, num_seats, starting);
            self.phase = Phase::Playing;
            let discard_start = *self.hole.discard.last().expect("discard seeded at deal");
            vec![
                Event::HoleDealt {
                    hole: self.hole_number,
                    starting_seat: starting,
                    discard_start,
                },
                Event::TurnStarted { seat: starting },
            ]
        }
    }
}
