use golf_engine::bot::Difficulty;
use golf_engine::{Action, Event, PlayerView, Seat};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[serde(tag = "type")]
#[ts(export)]
pub enum ClientMsg {
    /// Open a new room; the sender becomes seat 0 and host.
    CreateRoom {
        name: String,
    },
    /// Join an existing room by code.
    JoinRoom {
        code: String,
        name: String,
    },
    /// Reclaim a seat after a disconnect using the session token.
    Rejoin {
        code: String,
        token: String,
    },
    /// Host only, lobby only.
    AddBot {
        difficulty: Difficulty,
    },
    /// Host only. In the lobby the seat (bot or human) is removed; mid-game
    /// a kicked human's seat is handed to a bot so the match can continue.
    RemoveSeat {
        seat: Seat,
    },
    /// Host only; also serves as "rematch" once a match is over.
    StartMatch {
        holes: u8,
    },
    /// A game action for the sender's seat.
    Act {
        action: Action,
    },
    /// A quick reaction shown to the whole table. Rate-limited server-side.
    Emote {
        emote: Emote,
    },
    Ping,
}

/// Fixed emote palette — a closed set so clients never render untrusted text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Emote {
    Wave,
    Laugh,
    Cry,
    Fire,
    Clap,
    Zzz,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type")]
#[ts(export)]
pub enum ServerMsg {
    /// Sent once when a seat is (re)bound to this connection.
    RoomJoined {
        code: String,
        seat: Seat,
        token: String,
    },
    Lobby {
        state: LobbyState,
    },
    /// Public animation stream; always followed by a `State` snapshot.
    Events {
        events: Vec<Event>,
    },
    /// Personalized, redacted snapshot — the client's whole truth.
    State {
        view: PlayerView,
    },
    /// A player's reaction, fanned out to everyone at the table.
    Emote {
        seat: Seat,
        emote: Emote,
    },
    Error {
        code: ErrorCode,
        message: String,
    },
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LobbyState {
    pub code: String,
    pub seats: Vec<LobbySeat>,
    pub host: Seat,
    pub you: Seat,
    pub started: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LobbySeat {
    pub name: String,
    pub difficulty: Option<Difficulty>,
    pub connected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ErrorCode {
    RoomNotFound,
    RoomFull,
    MatchAlreadyStarted,
    MatchNotStarted,
    NotHost,
    BadToken,
    BadAction,
    BadRequest,
    /// The host removed you from the table. Terminal: the seat token is void.
    Kicked,
}
