use crate::card::Card;
use crate::score::optimal_pairing;
use crate::state::{MatchState, Phase, Seat, SeatKind, TurnState};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Viewer {
    Seat(Seat),
    Spectator,
}

/// What one viewer can see of a single card slot.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type")]
#[ts(export)]
pub enum SlotView {
    /// Face-down and unknown to this viewer.
    Hidden,
    /// Face-down, but the viewer owns it and knows it (bottom row at deal).
    Peeked { card: Card },
    /// Public and frozen.
    FaceUp { card: Card },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SeatView {
    pub name: String,
    pub is_bot: bool,
    pub grid: Vec<SlotView>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ActionKind {
    DrawFromDeck,
    TakeDiscard,
    Flip,
    Swap,
    Keep,
    NextHole,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type")]
#[ts(export)]
pub enum PhaseView {
    Playing,
    HoleComplete {
        scores: Vec<i32>,
        /// Optimal pairing per seat, as slot-index pairs, for highlighting.
        pairings: Vec<Vec<(u8, u8)>>,
    },
    MatchComplete {
        winners: Vec<Seat>,
    },
}

/// Everything one viewer may know about the match. Serializing an opponent's
/// or spectator's view never leaks a `Hidden` card.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PlayerView {
    /// The seat this view was redacted for; `None` = spectator.
    pub viewer: Option<Seat>,
    pub seats: Vec<SeatView>,
    pub deck_len: u8,
    pub discard_top: Option<Card>,
    pub discard_len: u8,
    pub current: Seat,
    pub turn: TurnState,
    pub hole_number: u8,
    pub holes_total: u8,
    pub totals: Vec<i32>,
    /// Completed holes: `score_history[hole][seat]`.
    pub score_history: Vec<Vec<i32>>,
    pub phase: PhaseView,
    /// Action kinds the viewer may take right now (empty when not their turn).
    pub legal_actions: Vec<ActionKind>,
}

impl MatchState {
    pub fn view(&self, viewer: Viewer) -> PlayerView {
        let viewer_seat = match viewer {
            Viewer::Seat(seat) => Some(seat),
            Viewer::Spectator => None,
        };
        let seats = self
            .config
            .seats
            .iter()
            .zip(&self.hole.players)
            .enumerate()
            .map(|(owner, (cfg, player))| SeatView {
                name: cfg.name.clone(),
                is_bot: matches!(cfg.kind, SeatKind::Bot { .. }),
                grid: player
                    .grid
                    .iter()
                    .map(|slot| {
                        if slot.face_up {
                            SlotView::FaceUp { card: slot.card }
                        } else if viewer_seat == Some(owner as Seat) && slot.known_to_owner {
                            SlotView::Peeked { card: slot.card }
                        } else {
                            SlotView::Hidden
                        }
                    })
                    .collect(),
            })
            .collect();

        let phase = match &self.phase {
            Phase::Playing => PhaseView::Playing,
            Phase::HoleComplete { scores } => PhaseView::HoleComplete {
                scores: scores.clone(),
                pairings: self
                    .hole
                    .players
                    .iter()
                    .map(|p| optimal_pairing(&p.cards()).1)
                    .collect(),
            },
            Phase::MatchComplete { winners } => PhaseView::MatchComplete {
                winners: winners.clone(),
            },
        };

        let legal_actions = match (&self.phase, viewer_seat) {
            (Phase::Playing, Some(seat)) if seat == self.hole.current => match self.hole.turn {
                TurnState::AwaitDraw => vec![ActionKind::DrawFromDeck, ActionKind::TakeDiscard],
                TurnState::AwaitFlip { .. } => vec![ActionKind::Flip],
                TurnState::AwaitResolve { .. } => vec![ActionKind::Swap, ActionKind::Keep],
            },
            (Phase::HoleComplete { .. }, Some(_)) => vec![ActionKind::NextHole],
            _ => Vec::new(),
        };

        PlayerView {
            viewer: viewer_seat,
            seats,
            deck_len: self.hole.deck.len() as u8,
            discard_top: self.hole.discard.last().copied(),
            discard_len: self.hole.discard.len() as u8,
            current: self.hole.current,
            turn: self.hole.turn,
            hole_number: self.hole_number,
            holes_total: self.config.holes,
            totals: self.totals.clone(),
            score_history: self.score_history.clone(),
            phase,
            legal_actions,
        }
    }
}
