mod easy;
mod heuristic;

use crate::action::Action;
use crate::view::PlayerView;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use easy::EasyBot;
pub use heuristic::HeuristicBot;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

/// Bots decide from a redacted [`PlayerView`] only, so they are structurally
/// incapable of cheating, and the same code runs in WASM and on the server.
pub trait Bot: Send {
    /// Choose the next action for the viewer seat of `view`. Only called when
    /// it is this bot's turn; must return a legal action.
    fn choose(&mut self, view: &PlayerView, rng: &mut dyn RngCore) -> Action;
}

pub fn make_bot(difficulty: Difficulty) -> Box<dyn Bot> {
    match difficulty {
        Difficulty::Easy => Box::new(EasyBot),
        Difficulty::Medium => Box::new(HeuristicBot::medium()),
        Difficulty::Hard => Box::new(HeuristicBot::hard()),
    }
}
