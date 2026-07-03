use golf_engine::score::{optimal_pairing, pair_value, pairs_with, score_hand};
use golf_engine::{Card, Rank, Suit};

/// Build a 6-card hand from rank shorthand; suits are assigned round-robin so
/// duplicate ranks are distinct physical cards.
fn hand(ranks: [Rank; 6]) -> [Card; 6] {
    std::array::from_fn(|i| Card {
        rank: ranks[i],
        suit: Suit::ALL[i % 4],
    })
}

use Rank::*;

#[test]
fn no_pairs_sums_face_values() {
    assert_eq!(score_hand(&hand([Two, Three, Four, Five, Six, Seven])), 27);
    // Face cards are all worth 10.
    assert_eq!(score_hand(&hand([Ten, Jack, Queen, King, Two, Three])), 45);
}

#[test]
fn natural_pair_scores_zero() {
    assert_eq!(score_hand(&hand([King, King, Two, Three, Four, Five])), 14);
    assert_eq!(
        score_hand(&hand([Seven, Seven, Seven, Seven, Two, Three])),
        5
    );
    assert_eq!(score_hand(&hand([Four, Four, Five, Five, Four, Two])), 6);
}

#[test]
fn ace_pairs_with_anything_for_minus_two() {
    // Ace absorbs the king: −2 + 3+4+5+6 = 16.
    assert_eq!(score_hand(&hand([Ace, King, Three, Four, Five, Six])), 16);
    // Ace should absorb the most expensive card, not just any card.
    assert_eq!(score_hand(&hand([Ace, King, Two, Three, Four, Five])), 12);
}

#[test]
fn unpaired_ace_scores_minus_two() {
    // A + 5-5 pair + 3+2+9: ace best used on the 9: −2 + 5 = 3.
    assert_eq!(score_hand(&hand([Ace, Five, Five, Three, Two, Nine])), 3);
}

#[test]
fn two_aces_prefer_absorbing_over_pairing_each_other() {
    // A-K (−2) + A-Q (−2) + 5 + 2 = 3, beats AA-pair (−2) or unpaired (−4) + 27.
    assert_eq!(score_hand(&hand([Ace, Ace, King, Queen, Five, Two])), 3);
    // Best: 9-9 pair naturally (0) while the aces absorb the 5 and 2 (−4).
    assert_eq!(score_hand(&hand([Ace, Ace, Nine, Nine, Five, Two])), -4);
}

#[test]
fn many_aces() {
    // Three aces: absorb K, Q, and the 2 → −6.
    assert_eq!(score_hand(&hand([Ace, Ace, Ace, King, Queen, Two])), -6);
    // Four aces: two absorb K and Q (−4), two stay unpaired (−4) → −8.
    assert_eq!(score_hand(&hand([Ace, Ace, Ace, Ace, King, Queen])), -8);
    // Six... four aces and two low cards: absorb both (−4) + unpaired aces (−4) → −8.
    assert_eq!(score_hand(&hand([Ace, Ace, Ace, Ace, Two, Two])), -8);
}

#[test]
fn ace_vs_natural_pair_is_globally_optimal() {
    // A,K,K,Q,J,2: K-K (0) + A-Q (−2) + 10 + 2 = 10
    // vs A-K (−2) + K,Q,J,2 unpaired (32) = 30. Optimizer must pick the first.
    assert_eq!(score_hand(&hand([Ace, King, King, Queen, Jack, Two])), 10);
}

#[test]
fn optimal_pairing_returns_the_pairs() {
    // Unique optimum: K-K (0) + A-Q (−2) + 9 + 2 = 9.
    let cards = hand([Ace, King, King, Queen, Nine, Two]);
    let (score, pairs) = optimal_pairing(&cards);
    assert_eq!(score, 9);
    let mut normalized: Vec<(u8, u8)> = pairs.iter().map(|&(a, b)| (a.min(b), a.max(b))).collect();
    normalized.sort();
    assert_eq!(normalized, vec![(0, 3), (1, 2)]); // A-Q and K-K
}

/// Independent cross-check: enumerate disjoint pair sets combinatorially
/// (a completely different algorithm from the engine's recursion) and compare
/// on a few thousand random hands.
#[test]
fn matches_independent_brute_force() {
    fn brute_force(cards: &[Card; 6]) -> i32 {
        let all_pairs: Vec<(usize, usize)> = (0..6)
            .flat_map(|a| (a + 1..6).map(move |b| (a, b)))
            .collect(); // 15 candidate pairs
        let mut best = i32::MAX;
        // Every subset of the 15 pairs; keep those that are disjoint & legal.
        for mask in 0u32..(1 << 15) {
            let chosen: Vec<(usize, usize)> = all_pairs
                .iter()
                .enumerate()
                .filter(|(i, _)| mask & (1 << i) != 0)
                .map(|(_, &p)| p)
                .collect();
            let mut used = [false; 6];
            let mut ok = true;
            let mut score = 0;
            for &(a, b) in &chosen {
                if used[a] || used[b] || !pairs_with(cards[a], cards[b]) {
                    ok = false;
                    break;
                }
                used[a] = true;
                used[b] = true;
                score += pair_value(cards[a], cards[b]);
            }
            if !ok {
                continue;
            }
            for (i, &u) in used.iter().enumerate() {
                if !u {
                    score += cards[i].rank.unpaired_value();
                }
            }
            best = best.min(score);
        }
        best
    }

    // Deterministic pseudo-random hands without pulling in a test-only RNG.
    let mut seed: u64 = 0x1234_5678_9abc_def0;
    let mut next = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) as usize
    };
    for _ in 0..400 {
        // Partial Fisher–Yates: 6 distinct cards, like a real deal.
        let mut deck = golf_engine::card::standard_deck();
        for i in 0..6 {
            let j = i + next() % (52 - i);
            deck.swap(i, j);
        }
        let cards: [Card; 6] = std::array::from_fn(|i| deck[i]);
        assert_eq!(
            score_hand(&cards),
            brute_force(&cards),
            "disagreement on {cards:?}"
        );
    }
}
