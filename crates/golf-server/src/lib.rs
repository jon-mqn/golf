pub mod protocol;
pub mod registry;
pub mod room;
mod r#static;
mod ws;

use axum::routing::get;
use axum::Router;
use std::sync::Arc;
use tower_http::compression::CompressionLayer;

#[derive(Default)]
pub struct AppState {
    pub registry: Arc<registry::Registry>,
}

pub fn app() -> Router {
    Router::new()
        .route("/ws", get(ws::ws_handler))
        .route("/healthz", get(|| async { "ok" }))
        .fallback(r#static::static_handler)
        // Gzip/brotli cuts the WASM+JS payload to roughly a third; a no-op
        // for the WebSocket upgrade.
        .layer(CompressionLayer::new())
        .with_state(Arc::new(AppState::default()))
}
