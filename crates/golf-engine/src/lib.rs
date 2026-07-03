//! Golf card game engine: pure, deterministic, no I/O.
//!
//! The variant implemented here:
//! - 2–4 players, 6 cards each in a 2×3 grid (slots 0–2 top row, 3–5 bottom row).
//! - All cards start face-down; each player privately knows their bottom row.
//! - Turn: draw from deck or take the discard → flip one of your face-down
//!   cards → swap the drawn card in, or keep your card. The slot is then
//!   face-up and frozen. Exactly 6 turns per player per hole.
//! - Scoring: any two cards of equal rank pair for 0. Aces are wild: they pair
//!   with anything for −2, or score −2 unpaired. Other unpaired cards score
//!   face value (J/Q/K = 10). The engine finds the optimal pairing.

pub mod action;
mod apply;
pub mod bot;
pub mod card;
pub mod event;
pub mod score;
pub mod state;
pub mod view;

pub use action::{Action, RuleError};
pub use card::{Card, Rank, Suit};
pub use event::Event;
pub use state::{
    DrawSource, MatchConfig, MatchState, Phase, Seat, SeatConfig, SeatKind, SlotIdx, TurnState,
    GRID_SIZE,
};
pub use view::{ActionKind, PhaseView, PlayerView, SeatView, SlotView, Viewer};
