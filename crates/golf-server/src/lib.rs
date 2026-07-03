pub mod protocol;
pub mod registry;
pub mod room;
mod r#static;
mod ws;

use axum::routing::get;
use axum::Router;
use std::sync::Arc;

#[derive(Default)]
pub struct AppState {
    pub registry: Arc<registry::Registry>,
}

pub fn app() -> Router {
    Router::new()
        .route("/ws", get(ws::ws_handler))
        .route("/healthz", get(|| async { "ok" }))
        .fallback(r#static::static_handler)
        .with_state(Arc::new(AppState::default()))
}
