# Multi-stage build: compiles the WASM engine, the frontend, and the server
# inside the container, so `fly deploy` needs no local toolchain. The result
# runs the single self-contained golf-server binary.

FROM rust:1-bookworm AS build

# Node 22 (frontend build) and wasm-pack, matching the versions used in dev.
RUN curl -fsSL https://nodejs.org/dist/v22.17.0/node-v22.17.0-linux-x64.tar.xz \
      | tar -xJ -C /usr/local --strip-components=1 \
 && curl -fsSL https://github.com/rustwasm/wasm-pack/releases/download/v0.13.1/wasm-pack-v0.13.1-x86_64-unknown-linux-musl.tar.gz \
      | tar -xz -C /usr/local/bin --strip-components=1 wasm-pack-v0.13.1-x86_64-unknown-linux-musl/wasm-pack \
 && rustup target add wasm32-unknown-unknown

WORKDIR /app
COPY . .

# Engine → WASM, then the frontend (embeds into the server binary next).
RUN wasm-pack build crates/golf-wasm --release --target web \
      --out-dir ../../web/src/lib/wasm --no-pack
RUN cd web && npm ci && npm run build

RUN cargo build --release -p golf-server

FROM debian:bookworm-slim
COPY --from=build /app/target/release/golf-server /usr/local/bin/golf-server
ENV PORT=8080
EXPOSE 8080
CMD ["golf-server"]
