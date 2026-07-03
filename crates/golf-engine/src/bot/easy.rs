use crate::action::Action;
use crate::bot::Bot;
use crate::card::Rank;
use crate::state::TurnState;
use crate::view::{PlayerView, SlotView};
use rand::seq::IndexedRandom;
use rand::RngCore;

/// Simple threshold bot: takes obviously good discards, flips at random,
/// swaps when the drawn card is plainly lower.
pub struct EasyBot;

impl Bot for EasyBot {
    fn choose(&mut self, view: &PlayerView, rng: &mut dyn RngCore) -> Action {
        let me = view.viewer.expect("bots always view from a seat") as usize;
        let grid = &view.seats[me].grid;
        match view.turn {
            TurnState::AwaitDraw => {
                let top = view
                    .discard_top
                    .expect("discard is never empty at draw time");
                let pairs_known = grid.iter().any(|s| match s {
                    SlotView::Peeked { card } | SlotView::FaceUp { card } => card.rank == top.rank,
                    SlotView::Hidden => false,
                });
                if top.rank == Rank::Ace || pairs_known || top.rank.unpaired_value() <= 3 {
                    Action::TakeDiscard
                } else {
                    Action::DrawFromDeck
                }
            }
            TurnState::AwaitFlip { .. } => {
                let face_down: Vec<u8> = grid
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| !matches!(s, SlotView::FaceUp { .. }))
                    .map(|(i, _)| i as u8)
                    .collect();
                let slot = *face_down
                    .choose(rng)
                    .expect("turn implies a face-down slot");
                Action::Flip { slot }
            }
            TurnState::AwaitResolve { drawn, flipped, .. } => {
                let SlotView::FaceUp { card: mine } = grid[flipped as usize] else {
                    unreachable!("flipped slot is face up");
                };
                if drawn.rank.unpaired_value() < mine.rank.unpaired_value() {
                    Action::Swap
                } else {
                    Action::Keep
                }
            }
        }
    }
}
