use crate::state::SlotIdx;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type")]
#[ts(export)]
pub enum Action {
    /// Flip the top card of the deck (public).
    DrawFromDeck,
    /// Take the top card of the discard pile.
    TakeDiscard,
    /// Flip one of your own face-down cards.
    Flip { slot: SlotIdx },
    /// Place the drawn card in the flipped slot; the flipped card is discarded.
    Swap,
    /// Keep the flipped card; the drawn card is discarded.
    Keep,
    /// Advance from the hole scoreboard to the next hole (or end the match).
    NextHole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error, Serialize, Deserialize, TS)]
#[serde(tag = "type")]
#[ts(export)]
pub enum RuleError {
    #[error("not your turn")]
    NotYourTurn,
    #[error("that action is not valid right now")]
    WrongPhase,
    #[error("no such seat")]
    BadSeat,
    #[error("no such card slot")]
    BadSlot,
    #[error("that card is already face up")]
    SlotFrozen,
    #[error("the match is over")]
    MatchOver,
}
