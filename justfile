# Golf — dev workflow. Requires: cargo, node/npm, wasm-pack, just (all on PATH).

default: test

# Build the engine to WASM for the frontend (dev profile).
wasm:
    wasm-pack build crates/golf-wasm --dev --target web --out-dir ../../web/src/lib/wasm --no-pack

# Regenerate the TypeScript protocol types from the Rust structs.
bindings:
    TS_RS_EXPORT_DIR={{justfile_directory()}}/web/src/lib/protocol cargo test -p golf-engine export_bindings

# Run the API server (:8080) and Vite dev server (:5173) together.
dev: wasm bindings
    #!/usr/bin/env bash
    trap 'kill 0' EXIT
    cargo run -p golf-server &
    (cd web && npm run dev) &
    wait

# Full test suite.
test:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    cd web && npm run check

# Release build: wasm → vite → single server binary with embedded assets.
build: bindings
    wasm-pack build crates/golf-wasm --release --target web --out-dir ../../web/src/lib/wasm --no-pack
    cd web && npm run build
    cargo build --release -p golf-server
