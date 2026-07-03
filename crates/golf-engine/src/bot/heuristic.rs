use crate::action::Action;
use crate::bot::Bot;
use crate::card::{Card, Rank, Suit};
use crate::score::{pair_value, pairs_with};
use crate::state::{SlotIdx, TurnState};
use crate::view::{PlayerView, SlotView};
use rand::seq::IndexedRandom;
use rand::RngCore;

/// Expected unpaired cost of an unknown card, tuned slightly below the raw
/// mean (≈6.3) to account for its chance of pairing later.
const UNKNOWN_EV: f64 = 5.1;

/// Expected-value bot. Evaluates hand knowledge (known cards optimally
/// paired, unknown slots at their expected cost) and picks the action that
/// minimizes the evaluation. `Hard` additionally conditions the unknown-card
/// distribution on every card currently visible on the table.
pub struct HeuristicBot {
    card_counting: bool,
}

impl HeuristicBot {
    pub fn medium() -> Self {
        HeuristicBot {
            card_counting: false,
        }
    }

    pub fn hard() -> Self {
        HeuristicBot {
            card_counting: true,
        }
    }
}

#[derive(Clone, Copy)]
struct HandSlot {
    card: Option<Card>,
    frozen: bool,
}

fn my_hand(view: &PlayerView) -> [HandSlot; 6] {
    let me = view.viewer.expect("bots always view from a seat") as usize;
    let grid = &view.seats[me].grid;
    std::array::from_fn(|i| match grid[i] {
        SlotView::Hidden => HandSlot {
            card: None,
            frozen: false,
        },
        SlotView::Peeked { card } => HandSlot {
            card: Some(card),
            frozen: false,
        },
        SlotView::FaceUp { card } => HandSlot {
            card: Some(card),
            frozen: true,
        },
    })
}

/// Optimal score of the known cards plus expected cost of the unknowns.
fn eval(hand: &[HandSlot; 6], unknown_ev: f64) -> f64 {
    let known: Vec<Card> = hand.iter().filter_map(|s| s.card).collect();
    let unknowns = hand.len() - known.len();
    best_known(&known) as f64 + unknown_ev * unknowns as f64
}

fn best_known(cards: &[Card]) -> i32 {
    let Some((&first, rest)) = cards.split_first() else {
        return 0;
    };
    let mut best = first.rank.unpaired_value() + best_known(rest);
    for i in 0..rest.len() {
        if pairs_with(first, rest[i]) {
            let mut remaining = rest.to_vec();
            remaining.remove(i);
            best = best.min(pair_value(first, rest[i]) + best_known(&remaining));
        }
    }
    best
}

/// Best improvement achievable by placing `card` into a non-frozen slot:
/// `(gain, slot)`, where gain > 0 means the hand gets better.
fn best_placement(hand: &[HandSlot; 6], card: Card, unknown_ev: f64) -> (f64, SlotIdx) {
    let base = eval(hand, unknown_ev);
    let mut best_gain = f64::NEG_INFINITY;
    let mut best_slot = 0;
    for (i, slot) in hand.iter().enumerate() {
        if slot.frozen {
            continue;
        }
        let mut with = *hand;
        with[i] = HandSlot {
            card: Some(card),
            frozen: true,
        };
        let gain = base - eval(&with, unknown_ev);
        if gain > best_gain {
            best_gain = gain;
            best_slot = i as SlotIdx;
        }
    }
    (best_gain, best_slot)
}

impl HeuristicBot {
    /// Probability of each rank on a blind deck draw.
    fn rank_probs(&self, view: &PlayerView) -> [f64; 13] {
        if !self.card_counting {
            return [1.0 / 13.0; 13];
        }
        // Count every card currently visible anywhere on the table.
        let mut remaining = [4i32; 13];
        let mut saw = |card: Card| {
            let idx = Rank::ALL.iter().position(|&r| r == card.rank).unwrap();
            remaining[idx] = (remaining[idx] - 1).max(0);
        };
        for seat in &view.seats {
            for slot in &seat.grid {
                match slot {
                    SlotView::Peeked { card } | SlotView::FaceUp { card } => saw(*card),
                    SlotView::Hidden => {}
                }
            }
        }
        if let Some(top) = view.discard_top {
            saw(top);
        }
        match view.turn {
            TurnState::AwaitFlip { drawn, .. } | TurnState::AwaitResolve { drawn, .. } => {
                saw(drawn)
            }
            TurnState::AwaitDraw => {}
        }
        let total: i32 = remaining.iter().sum();
        std::array::from_fn(|i| remaining[i] as f64 / total as f64)
    }

    fn unknown_ev(&self, probs: &[f64; 13]) -> f64 {
        if !self.card_counting {
            return UNKNOWN_EV;
        }
        let mean: f64 = Rank::ALL
            .iter()
            .zip(probs)
            .map(|(r, p)| p * r.unpaired_value() as f64)
            .sum();
        // Same pairing-potential discount as the fixed constant applies.
        mean - 1.2
    }
}

impl Bot for HeuristicBot {
    fn choose(&mut self, view: &PlayerView, rng: &mut dyn RngCore) -> Action {
        let hand = my_hand(view);
        let probs = self.rank_probs(view);
        let ev = self.unknown_ev(&probs);
        match view.turn {
            TurnState::AwaitDraw => {
                let top = view
                    .discard_top
                    .expect("discard is never empty at draw time");
                let (top_gain, _) = best_placement(&hand, top, ev);
                // Expected gain of a blind draw: for each rank, the best
                // placement gain if positive (junk draws cost ~nothing — we
                // flip and keep).
                let deck_gain: f64 = Rank::ALL
                    .iter()
                    .zip(&probs)
                    .map(|(&rank, p)| {
                        let hypothetical = Card {
                            rank,
                            suit: Suit::Spades,
                        };
                        p * best_placement(&hand, hypothetical, ev).0.max(0.0)
                    })
                    .sum();
                if top_gain > 0.0 && top_gain >= deck_gain {
                    Action::TakeDiscard
                } else {
                    Action::DrawFromDeck
                }
            }
            TurnState::AwaitFlip { drawn, .. } => {
                let (gain, slot) = best_placement(&hand, drawn, ev);
                if gain > 0.0 {
                    return Action::Flip { slot };
                }
                // The drawn card is junk: flip something we are happy to
                // freeze. A known low card (or one pairing a known card)
                // freezes for free and preserves hidden slots as future swap
                // targets; otherwise gamble on an unknown slot.
                let known_free: Vec<(SlotIdx, Card)> = hand
                    .iter()
                    .enumerate()
                    .filter_map(|(i, s)| match (s.frozen, s.card) {
                        (false, Some(card)) => Some((i as SlotIdx, card)),
                        _ => None,
                    })
                    .collect();
                let safe = known_free
                    .iter()
                    .filter(|(i, card)| {
                        let pairs_other = hand.iter().enumerate().any(|(j, other)| {
                            j != *i as usize && other.card.is_some_and(|c| pairs_with(*card, c))
                        });
                        card.rank.unpaired_value() <= 4 || pairs_other
                    })
                    .min_by_key(|(_, card)| card.rank.unpaired_value());
                if let Some(&(slot, _)) = safe {
                    return Action::Flip { slot };
                }
                let unknown: Vec<SlotIdx> = hand
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.card.is_none())
                    .map(|(i, _)| i as SlotIdx)
                    .collect();
                if let Some(&slot) = unknown.choose(rng) {
                    return Action::Flip { slot };
                }
                // All remaining face-down cards are known and bad: flip the
                // least bad one (the resolve step may still swap it away).
                let slot = known_free
                    .iter()
                    .min_by_key(|(_, card)| card.rank.unpaired_value())
                    .expect("turn implies a face-down slot")
                    .0;
                Action::Flip { slot }
            }
            TurnState::AwaitResolve { drawn, flipped, .. } => {
                // The flipped card is now visible in the view (face-up), so
                // `hand` already reflects keeping it.
                let keep_eval = eval(&hand, ev);
                let mut with_swap = hand;
                with_swap[flipped as usize] = HandSlot {
                    card: Some(drawn),
                    frozen: true,
                };
                if eval(&with_swap, ev) < keep_eval {
                    Action::Swap
                } else {
                    Action::Keep
                }
            }
        }
    }
}
