use golf_engine::bot::{make_bot, Difficulty};
use golf_engine::{Action, MatchConfig, MatchState, Phase, SeatConfig, SeatKind, Viewer};
use rand::rngs::StdRng;
use rand::SeedableRng;

fn bot_config(difficulties: &[Difficulty], holes: u8) -> MatchConfig {
    MatchConfig {
        seats: difficulties
            .iter()
            .map(|&difficulty| SeatConfig {
                name: format!("{difficulty:?}"),
                kind: SeatKind::Bot { difficulty },
            })
            .collect(),
        holes,
    }
}

/// Run one all-bot match to completion, panicking on any illegal action.
fn play_match(difficulties: &[Difficulty], seed: u64) -> Vec<i32> {
    let mut m = MatchState::new(bot_config(difficulties, 9), seed).unwrap();
    let mut bots: Vec<_> = difficulties.iter().map(|&d| make_bot(d)).collect();
    let mut rng = StdRng::seed_from_u64(seed);
    loop {
        match &m.phase {
            Phase::MatchComplete { .. } => return m.totals.clone(),
            Phase::HoleComplete { .. } => {
                m.apply(0, Action::NextHole).unwrap();
            }
            Phase::Playing => {
                let seat = m.seat_to_act().unwrap();
                let view = m.view(Viewer::Seat(seat));
                let action = bots[seat as usize].choose(&view, &mut rng);
                m.apply(seat, action).unwrap_or_else(|e| {
                    panic!("bot {seat} chose illegal action {action:?}: {e} (seed {seed})")
                });
            }
        }
    }
}

#[test]
fn bots_never_choose_illegal_actions() {
    use Difficulty::*;
    for seed in 0..40 {
        play_match(&[Easy, Medium, Hard], seed);
        play_match(&[Hard, Easy], seed + 1000);
        play_match(&[Medium, Medium, Easy, Hard], seed + 2000);
    }
}

/// Statistical smoke test: over many matches the heuristic bot should
/// clearly outscore the threshold bot. Loose margin to avoid flakiness.
#[test]
fn medium_beats_easy_on_average() {
    let mut easy_total = 0i64;
    let mut medium_total = 0i64;
    for seed in 0..150 {
        // Alternate seats so neither side gets the first-player edge.
        let totals = play_match(&[Difficulty::Medium, Difficulty::Easy], seed);
        medium_total += totals[0] as i64;
        easy_total += totals[1] as i64;
        let totals = play_match(&[Difficulty::Easy, Difficulty::Medium], seed + 5000);
        easy_total += totals[0] as i64;
        medium_total += totals[1] as i64;
    }
    assert!(
        medium_total < easy_total,
        "medium ({medium_total}) should beat easy ({easy_total}) over 300 matches"
    );
}
