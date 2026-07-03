//! wasm-bindgen wrapper around `golf-engine` for fully in-browser play
//! (local pass-and-play and vs-bots). JSON strings cross the JS boundary;
//! the TypeScript types are generated from the same Rust structs via ts-rs.

use golf_engine::bot::{make_bot, Bot};
use golf_engine::{Action, MatchConfig, MatchState, SeatKind, Viewer};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmMatch {
    state: MatchState,
    bots: Vec<Option<Box<dyn Bot>>>,
    bot_rng: ChaCha8Rng,
}

#[wasm_bindgen]
impl WasmMatch {
    #[wasm_bindgen(constructor)]
    pub fn new(config_json: &str, seed: u64) -> Result<WasmMatch, JsError> {
        let config: MatchConfig = serde_json::from_str(config_json)?;
        let bots = config
            .seats
            .iter()
            .map(|seat| match seat.kind {
                SeatKind::Bot { difficulty } => Some(make_bot(difficulty)),
                SeatKind::Human => None,
            })
            .collect();
        let state = MatchState::new(config, seed).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(WasmMatch {
            state,
            bots,
            bot_rng: ChaCha8Rng::seed_from_u64(seed ^ 0x9e37_79b9_7f4a_7c15),
        })
    }

    /// Apply a player action; returns the produced events as JSON.
    pub fn apply(&mut self, seat: u8, action_json: &str) -> Result<String, JsError> {
        let action: Action = serde_json::from_str(action_json)?;
        let events = self
            .state
            .apply(seat, action)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(serde_json::to_string(&events)?)
    }

    /// Redacted view for a seat (or the spectator view), as JSON.
    pub fn view(&self, seat: Option<u8>) -> Result<String, JsError> {
        let viewer = seat.map(Viewer::Seat).unwrap_or(Viewer::Spectator);
        Ok(serde_json::to_string(&self.state.view(viewer))?)
    }

    pub fn seat_to_act(&self) -> Option<u8> {
        self.state.seat_to_act()
    }

    pub fn is_bot(&self, seat: u8) -> bool {
        self.bots
            .get(seat as usize)
            .is_some_and(|bot| bot.is_some())
    }

    /// Let the bot whose turn it is take ONE action (draw, flip, or resolve),
    /// so the frontend can pace animations between steps. Returns events JSON.
    pub fn bot_step(&mut self) -> Result<String, JsError> {
        let seat = self
            .state
            .seat_to_act()
            .ok_or_else(|| JsError::new("no seat to act"))?;
        let bot = self.bots[seat as usize]
            .as_mut()
            .ok_or_else(|| JsError::new("current seat is not a bot"))?;
        let view = self.state.view(Viewer::Seat(seat));
        let action = bot.choose(&view, &mut self.bot_rng);
        let events = self
            .state
            .apply(seat, action)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(serde_json::to_string(&events)?)
    }
}
