# Golf — the card game

Six cards, six turns, nine holes. Lowest score wins the round.

A web-based version of the card game Golf, playable on phones and desktops:

- **Pass & play** — several humans sharing one device, with a privacy screen
  between turns.
- **Play the bots** — Easy / Medium / Hard opponents, fully in-browser.
- **Online tables** — create a table, share the 4-letter code or the
  `/room/CODE` link; mix friends and bots. Refreshing or losing signal
  mid-game reconnects you to your seat, emotes let the table heckle each
  other, and the host can remove players (mid-game a bot takes over the
  seat so the hole finishes).

## Rules (this variant)

Each player gets 6 cards in a 2×3 grid, all face-down; you privately know your
bottom row. On your turn: take the face-up discard **or** flip the top of the
deck, then flip one of your own cards, then either **swap** the drawn card into
that slot or **keep** your card (discarding the other). A face-up card is
frozen forever, so the hole ends after six turns each.

Scoring: any two cards of the same rank pair for **0**. Aces are wild — an ace
pairs with any card for **−2**, or scores −2 alone. Unpaired cards cost face
value (J/Q/K = 10). The engine finds your best pairing automatically. Lowest
total after the last hole wins.

## Architecture

| Piece | What it is |
| --- | --- |
| `crates/golf-engine` | Pure, deterministic rules engine: state machine, optimal-pairing scorer, per-viewer redacted views, bots. |
| `crates/golf-wasm` | wasm-bindgen wrapper so local modes and bots run entirely in the browser. |
| `crates/golf-server` | axum WebSocket server: one actor task per room, redacted state snapshots, reconnect tokens, embedded static frontend. |
| `web/` | Svelte 5 + TypeScript UI. Protocol types are generated from the Rust structs with ts-rs (`web/src/lib/protocol`, committed). |

One `GameSession` interface drives the table UI in both modes: `LocalSession`
(WASM engine) and `OnlineSession` (WebSocket).

## Development

Requirements: Rust (stable, with the `wasm32-unknown-unknown` target),
Node 22+, [`wasm-pack`](https://rustwasm.github.io/wasm-pack/) and
[`just`](https://github.com/casey/just).

```sh
just wasm       # build the engine to web/src/lib/wasm
just dev        # golf-server on :8080 + Vite on :5173 (proxies /ws)
just test       # fmt + clippy + cargo tests + svelte-check
just bindings   # regenerate TS protocol types after touching Rust types
just build      # release: single self-contained binary with embedded UI
```

Play a hole in the terminal (great for rules tinkering):

```sh
cargo run -p golf-engine --example cli               # you vs a Medium bot
cargo run -p golf-engine --example cli -- --humans 0 --bots medium,hard
```

## Deploying

`just build` produces `target/release/golf-server` with the entire frontend
embedded — copy it anywhere and run it (`PORT` env var, default 8080). Rooms
live in memory and are collected 30 minutes after the last player disconnects.
