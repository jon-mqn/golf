use crate::bot::Difficulty;
use crate::card::{standard_deck, Card};
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub type Seat = u8;
pub type SlotIdx = u8;

/// Six cards per player: slots 0–2 are the top row (unknown to everyone at
/// deal), slots 3–5 the bottom row (privately known to the owner).
pub const GRID_SIZE: usize = 6;
pub const BOTTOM_ROW: std::ops::Range<usize> = 3..6;

pub const MIN_PLAYERS: usize = 2;
pub const MAX_PLAYERS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type")]
#[ts(export)]
pub enum SeatKind {
    Human,
    Bot { difficulty: Difficulty },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SeatConfig {
    pub name: String,
    pub kind: SeatKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MatchConfig {
    pub seats: Vec<SeatConfig>,
    pub holes: u8,
}

impl MatchConfig {
    pub fn nine_holes(seats: Vec<SeatConfig>) -> Self {
        MatchConfig { seats, holes: 9 }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("player count must be between {MIN_PLAYERS} and {MAX_PLAYERS}")]
    BadPlayerCount,
    #[error("number of holes must be at least 1")]
    NoHoles,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slot {
    pub card: Card,
    /// Face-up cards are public and frozen: they can never be flipped or
    /// swapped again.
    pub face_up: bool,
    /// The owner privately knows this face-down card (bottom row at deal).
    pub known_to_owner: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub grid: Vec<Slot>,
}

impl PlayerState {
    pub fn all_face_up(&self) -> bool {
        self.grid.iter().all(|s| s.face_up)
    }

    pub fn cards(&self) -> [Card; GRID_SIZE] {
        std::array::from_fn(|i| self.grid[i].card)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum DrawSource {
    Deck,
    Discard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type")]
#[ts(export)]
pub enum TurnState {
    AwaitDraw,
    AwaitFlip {
        drawn: Card,
        source: DrawSource,
    },
    AwaitResolve {
        drawn: Card,
        source: DrawSource,
        flipped: SlotIdx,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoleState {
    pub deck: Vec<Card>,
    pub discard: Vec<Card>,
    pub players: Vec<PlayerState>,
    pub current: Seat,
    pub turn: TurnState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Playing,
    HoleComplete { scores: Vec<i32> },
    MatchComplete { winners: Vec<Seat> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchState {
    pub config: MatchConfig,
    /// 1-based hole counter.
    pub hole_number: u8,
    pub totals: Vec<i32>,
    /// Per-hole scores for completed holes, `score_history[hole-1][seat]`.
    pub score_history: Vec<Vec<i32>>,
    pub hole: HoleState,
    pub phase: Phase,
    pub(crate) rng: ChaCha8Rng,
}

impl MatchState {
    pub fn new(config: MatchConfig, seed: u64) -> Result<Self, ConfigError> {
        let n = config.seats.len();
        if !(MIN_PLAYERS..=MAX_PLAYERS).contains(&n) {
            return Err(ConfigError::BadPlayerCount);
        }
        if config.holes == 0 {
            return Err(ConfigError::NoHoles);
        }
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let hole = deal_hole(&mut rng, n, 0);
        Ok(MatchState {
            totals: vec![0; n],
            score_history: Vec::new(),
            hole_number: 1,
            config,
            hole,
            phase: Phase::Playing,
            rng,
        })
    }

    pub fn num_seats(&self) -> usize {
        self.config.seats.len()
    }

    /// The seat that must act next, if any.
    pub fn seat_to_act(&self) -> Option<Seat> {
        match self.phase {
            Phase::Playing => Some(self.hole.current),
            _ => None,
        }
    }
}

pub(crate) fn deal_hole(rng: &mut ChaCha8Rng, num_players: usize, starting: Seat) -> HoleState {
    let mut deck = standard_deck();
    deck.shuffle(rng);
    let players = (0..num_players)
        .map(|_| PlayerState {
            grid: (0..GRID_SIZE)
                .map(|slot| Slot {
                    card: deck.pop().expect("deck holds enough for the deal"),
                    face_up: false,
                    known_to_owner: BOTTOM_ROW.contains(&slot),
                })
                .collect(),
        })
        .collect();
    let discard = vec![deck.pop().expect("deck holds enough for the deal")];
    HoleState {
        deck,
        discard,
        players,
        current: starting,
        turn: TurnState::AwaitDraw,
    }
}
