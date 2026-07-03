//! Terminal Golf, for validating rules and watching bots.
//!
//! Examples:
//!   cargo run -p golf-engine --example cli                    # you vs a Medium bot
//!   cargo run -p golf-engine --example cli -- --bots medium,hard --humans 0
//!   cargo run -p golf-engine --example cli -- --humans 2 --holes 3 --seed 42

use golf_engine::bot::{make_bot, Bot, Difficulty};
use golf_engine::{
    Action, Event, MatchConfig, MatchState, Phase, PlayerView, SeatConfig, SeatKind, SlotView,
    TurnState, Viewer,
};
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::io::{BufRead, Write};

fn main() {
    let mut humans = 1usize;
    let mut bot_list = vec![Difficulty::Medium];
    let mut holes = 9u8;
    let mut seed: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as u64;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--humans" => {
                humans = args[i + 1].parse().expect("--humans N");
                i += 2;
            }
            "--bots" => {
                bot_list = args[i + 1]
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| match s {
                        "easy" => Difficulty::Easy,
                        "medium" => Difficulty::Medium,
                        "hard" => Difficulty::Hard,
                        other => panic!("unknown difficulty {other:?}"),
                    })
                    .collect();
                i += 2;
            }
            "--holes" => {
                holes = args[i + 1].parse().expect("--holes N");
                i += 2;
            }
            "--seed" => {
                seed = args[i + 1].parse().expect("--seed N");
                i += 2;
            }
            other => panic!("unknown argument {other:?}"),
        }
    }

    let mut seats: Vec<SeatConfig> = (0..humans)
        .map(|i| SeatConfig {
            name: format!(
                "You{}",
                if humans > 1 {
                    (i + 1).to_string()
                } else {
                    String::new()
                }
            ),
            kind: SeatKind::Human,
        })
        .collect();
    for (i, &difficulty) in bot_list.iter().enumerate() {
        seats.push(SeatConfig {
            name: format!("Bot{} ({difficulty:?})", i + 1),
            kind: SeatKind::Bot { difficulty },
        });
    }

    let config = MatchConfig { seats, holes };
    let mut bots: Vec<Option<Box<dyn Bot>>> = config
        .seats
        .iter()
        .map(|s| match s.kind {
            SeatKind::Bot { difficulty } => Some(make_bot(difficulty)),
            SeatKind::Human => None,
        })
        .collect();
    let mut m = MatchState::new(config, seed).expect("valid config");
    let mut rng = StdRng::seed_from_u64(seed);
    println!("=== Golf — {holes} hole(s), seed {seed} ===");

    loop {
        match m.phase.clone() {
            Phase::MatchComplete { winners } => {
                println!("\n=== MATCH OVER ===");
                for (i, total) in m.totals.iter().enumerate() {
                    let mark = if winners.contains(&(i as u8)) {
                        " 🏆"
                    } else {
                        ""
                    };
                    println!("  {}: {}{}", m.config.seats[i].name, total, mark);
                }
                return;
            }
            Phase::HoleComplete { scores } => {
                println!("\n--- Hole {} complete ---", m.hole_number);
                for (i, s) in scores.iter().enumerate() {
                    println!(
                        "  {}: {:+} (total {:+})  {}",
                        m.config.seats[i].name,
                        s,
                        m.totals[i],
                        show_grid_row(&m.view(Viewer::Spectator), i)
                    );
                }
                if m.hole_number < m.config.holes {
                    prompt("press enter for the next hole");
                }
                m.apply(0, Action::NextHole).unwrap();
            }
            Phase::Playing => {
                let seat = m.seat_to_act().unwrap();
                let view = m.view(Viewer::Seat(seat));
                if let Some(bot) = bots[seat as usize].as_mut() {
                    let action = bot.choose(&view, &mut rng);
                    let events = m.apply(seat, action).unwrap();
                    print_events(&m, &events);
                } else {
                    human_turn(&mut m, seat, &view);
                }
            }
        }
    }
}

fn human_turn(m: &mut MatchState, seat: u8, view: &PlayerView) {
    match view.turn {
        TurnState::AwaitDraw => {
            print_table(view);
            let discard = view
                .discard_top
                .map(|c| c.to_string())
                .unwrap_or_else(|| "—".into());
            let choice = prompt_choice(
                &format!("[d] draw from deck   [t] take {discard} from discard"),
                &["d", "t"],
            );
            let action = if choice == "d" {
                Action::DrawFromDeck
            } else {
                Action::TakeDiscard
            };
            let events = m.apply(seat, action).unwrap();
            print_events(m, &events);
        }
        TurnState::AwaitFlip { drawn, .. } => {
            println!("  you are holding: {drawn}");
            let legal: Vec<String> = view.seats[seat as usize]
                .grid
                .iter()
                .enumerate()
                .filter(|(_, s)| !matches!(s, SlotView::FaceUp { .. }))
                .map(|(i, _)| (i + 1).to_string())
                .collect();
            let refs: Vec<&str> = legal.iter().map(String::as_str).collect();
            let choice = prompt_choice("flip which of your cards? (1-6)", &refs);
            let slot = choice.parse::<u8>().unwrap() - 1;
            let events = m.apply(seat, Action::Flip { slot }).unwrap();
            print_events(m, &events);
        }
        TurnState::AwaitResolve { drawn, flipped, .. } => {
            let SlotView::FaceUp { card: mine } = view.seats[seat as usize].grid[flipped as usize]
            else {
                unreachable!()
            };
            let choice = prompt_choice(
                &format!("[s] swap in {drawn} (discard your {mine})   [k] keep your {mine} (discard {drawn})"),
                &["s", "k"],
            );
            let action = if choice == "s" {
                Action::Swap
            } else {
                Action::Keep
            };
            let events = m.apply(seat, action).unwrap();
            print_events(m, &events);
        }
    }
}

fn print_table(view: &PlayerView) {
    let me = view.viewer.unwrap() as usize;
    println!(
        "\n━━━ Hole {}/{} — {}'s turn ━━━",
        view.hole_number, view.holes_total, view.seats[view.current as usize].name
    );
    for (i, seat) in view.seats.iter().enumerate() {
        if i == me {
            continue;
        }
        println!(
            "  {} (total {:+}): {}",
            seat.name,
            view.totals[i],
            show_grid_row(view, i)
        );
    }
    println!(
        "  deck: {} cards   discard: {}",
        view.deck_len,
        view.discard_top
            .map(|c| c.to_string())
            .unwrap_or_else(|| "—".into())
    );
    println!("  your cards (total {:+}):", view.totals[me]);
    let grid = &view.seats[me].grid;
    let row = |range: std::ops::Range<usize>| {
        range
            .map(|i| {
                format!(
                    "{}:{}",
                    i + 1,
                    match grid[i] {
                        SlotView::Hidden => "🂠 ".to_string(),
                        SlotView::Peeked { card } => format!("({card})"),
                        SlotView::FaceUp { card } => format!("[{card}]"),
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("  ")
    };
    println!("    {}", row(0..3));
    println!("    {}   (parentheses = only you can see it)", row(3..6));
}

fn show_grid_row(view: &PlayerView, seat: usize) -> String {
    view.seats[seat]
        .grid
        .iter()
        .map(|s| match s {
            SlotView::Hidden => "🂠".to_string(),
            SlotView::Peeked { card } => format!("({card})"),
            SlotView::FaceUp { card } => format!("[{card}]"),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn print_events(m: &MatchState, events: &[Event]) {
    for e in events {
        let name = |s: &u8| m.config.seats[*s as usize].name.clone();
        match e {
            Event::DrewFromDeck { seat, card } => {
                println!("  {} drew {card} from the deck", name(seat))
            }
            Event::TookDiscard { seat, card } => {
                println!("  {} took {card} from the discard", name(seat))
            }
            Event::Flipped { seat, slot, card } => {
                println!("  {} flipped card {} — it's {card}", name(seat), slot + 1)
            }
            Event::Swapped {
                seat,
                slot,
                placed,
                discarded,
            } => println!(
                "  {} swapped in {placed} at {} and discarded {discarded}",
                name(seat),
                slot + 1
            ),
            Event::Kept {
                seat, discarded, ..
            } => {
                println!("  {} kept their card and discarded {discarded}", name(seat))
            }
            Event::HoleEnded { .. } | Event::MatchEnded { .. } => {}
            Event::HoleDealt { hole, .. } => println!("\n═══ Hole {hole} dealt ═══"),
            Event::TurnStarted { .. } => {}
        }
    }
}

fn prompt(msg: &str) -> String {
    print!("{msg} > ");
    std::io::stdout().flush().unwrap();
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line).unwrap();
    line.trim().to_lowercase()
}

fn prompt_choice(msg: &str, allowed: &[&str]) -> String {
    loop {
        let ans = prompt(msg);
        if allowed.contains(&ans.as_str()) {
            return ans;
        }
        println!("  (choose one of: {})", allowed.join(", "));
    }
}
