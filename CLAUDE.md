# Golf — working notes

Web card game "Golf": Rust engine/server, Svelte 5 + TS frontend, WASM for
local play. See README.md for rules and layout.

## Commands

- `just test` — fmt + clippy (`-D warnings`) + cargo tests + svelte-check.
- `just wasm` — rebuild the WASM engine after touching `golf-engine`;
  the Vite dev server picks it up automatically.
- `just bindings` — regenerate `web/src/lib/protocol/*.ts` after changing any
  `#[derive(TS)]` type. The generated files are committed; CI fails on drift.
- `just dev` — server :8080 + Vite :5173 (Vite proxies `/ws`).
- Engine CLI: `cargo run -p golf-engine --example cli -- --humans 0 --bots medium,hard --holes 1 --seed 7`.

## Invariants to preserve

- The engine is the single source of rules; UI and server never re-implement
  game logic. All mutations go through `MatchState::apply`.
- Hidden information never leaves the engine except through
  `MatchState::view(viewer)`. Events are public-only by design (the deck flip
  is public in this variant). Tests in `golf-engine/tests/playouts.rs` and the
  server integration test enforce this — keep them passing.
- Bots decide from a redacted `PlayerView`, never from `MatchState`.
- Slots 0–2 are the top row (unknown), 3–5 the bottom row (owner-peeked at
  deal). Face-up ⇒ frozen.
- Deck can't exhaust and the discard is never empty at draw time with ≤4
  players — the engine asserts rather than handles these.

## Environment quirks (this machine)

- `cargo`, `node`, `npm`, `wasm-pack`, `just` live in `~/.cargo/bin` and
  `~/.local/bin` (Node 22 under `~/.local/opt/node-v22.17.0-linux-x64`).
- Server env knobs: `PORT`, `GOLF_BOT_STEP_MS` (bot pacing, tests use 5),
  `GOLF_ROOM_GC_SECS` (idle-room teardown, default 1800).
- Browser verification: Playwright + Chromium are installed in the session
  scratchpad, not the repo.
