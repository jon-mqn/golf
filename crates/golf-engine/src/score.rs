use crate::card::{Card, Rank};

/// Two cards pair if they share a rank, or if either is an ace (aces are wild).
pub fn pairs_with(a: Card, b: Card) -> bool {
    a.rank == b.rank || a.rank == Rank::Ace || b.rank == Rank::Ace
}

/// A pair containing an ace scores −2; a natural pair scores 0.
pub fn pair_value(a: Card, b: Card) -> i32 {
    if a.rank == Rank::Ace || b.rank == Rank::Ace {
        -2
    } else {
        0
    }
}

/// Minimum score of a hand under the optimal pairing.
pub fn score_hand(cards: &[Card; 6]) -> i32 {
    optimal_pairing(cards).0
}

/// Minimum score plus one pairing (slot index pairs) that achieves it.
///
/// Exhaustive search over all partial matchings of 6 cards (76 of them), so
/// the optimizer naturally leaves two aces unpaired (−4 beats −2) and pairs
/// an ace with the most expensive leftover card.
pub fn optimal_pairing(cards: &[Card; 6]) -> (i32, Vec<(u8, u8)>) {
    fn best(cards: &[Card; 6], remaining: u8) -> (i32, Vec<(u8, u8)>) {
        if remaining == 0 {
            return (0, Vec::new());
        }
        let first = remaining.trailing_zeros() as u8;
        let rest = remaining & !(1 << first);
        // Option 1: leave `first` unpaired.
        let (score, pairs) = best(cards, rest);
        let mut best_score = score + cards[first as usize].rank.unpaired_value();
        let mut best_pairs = pairs;
        // Option 2: pair `first` with each compatible remaining card.
        let mut m = rest;
        while m != 0 {
            let j = m.trailing_zeros() as u8;
            m &= !(1 << j);
            let (a, b) = (cards[first as usize], cards[j as usize]);
            if pairs_with(a, b) {
                let (score, mut pairs) = best(cards, rest & !(1 << j));
                let total = score + pair_value(a, b);
                if total < best_score {
                    pairs.push((first, j));
                    best_score = total;
                    best_pairs = pairs;
                }
            }
        }
        (best_score, best_pairs)
    }
    best(cards, 0b11_1111)
}
