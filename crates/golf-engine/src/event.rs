use crate::card::Card;
use crate::state::{Seat, SlotIdx};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Public record of something that happened. Events never contain hidden
/// information (the deck flip is public in this variant), so a single event
/// stream can be broadcast to every player and spectator unredacted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type")]
#[ts(export)]
pub enum Event {
    HoleDealt {
        hole: u8,
        starting_seat: Seat,
        discard_start: Card,
    },
    TurnStarted {
        seat: Seat,
    },
    DrewFromDeck {
        seat: Seat,
        card: Card,
    },
    TookDiscard {
        seat: Seat,
        card: Card,
    },
    Flipped {
        seat: Seat,
        slot: SlotIdx,
        card: Card,
    },
    Swapped {
        seat: Seat,
        slot: SlotIdx,
        placed: Card,
        discarded: Card,
    },
    Kept {
        seat: Seat,
        slot: SlotIdx,
        discarded: Card,
    },
    HoleEnded {
        hole: u8,
        scores: Vec<i32>,
        totals: Vec<i32>,
    },
    MatchEnded {
        totals: Vec<i32>,
        winners: Vec<Seat>,
    },
}
