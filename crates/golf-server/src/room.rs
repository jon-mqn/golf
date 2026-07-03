use crate::protocol::{ClientMsg, Emote, ErrorCode, LobbySeat, LobbyState, ServerMsg};
use crate::registry::Registry;
use futures_util::FutureExt;
use golf_engine::bot::{make_bot, Bot, Difficulty};
use golf_engine::{Event, MatchConfig, MatchState, Phase, Seat, SeatConfig, SeatKind, Viewer};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::Instant;

/// Delay between bot sub-actions so spectating humans can follow along.
fn bot_step_delay() -> Duration {
    std::env::var("GOLF_BOT_STEP_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_millis(700))
}
/// A room with no connections left is torn down after this long.
const DEFAULT_GC: Duration = Duration::from_secs(30 * 60);
/// In the lobby a disconnected (non-host) seat is held this long for a
/// rejoin before it's freed. Mid-game seats are held indefinitely.
const DEFAULT_LOBBY_GRACE: Duration = Duration::from_secs(90);
/// Minimum gap between emotes from one seat; extras are silently dropped.
const EMOTE_COOLDOWN: Duration = Duration::from_millis(1200);
/// Difficulty of the bot that takes over a seat when its human is kicked
/// mid-game.
const TAKEOVER_BOT: Difficulty = Difficulty::Medium;

const MAX_SEATS: usize = 4;

/// Identifies one WebSocket connection within a room. Stable across seat
/// removals, unlike seat indices.
pub type ConnId = u64;

#[derive(Debug)]
pub enum RoomCmd {
    Join {
        name: String,
        conn: mpsc::Sender<ServerMsg>,
        reply: oneshot::Sender<Result<ConnId, ErrorCode>>,
    },
    Rejoin {
        token: String,
        conn: mpsc::Sender<ServerMsg>,
        reply: oneshot::Sender<Result<ConnId, ErrorCode>>,
    },
    Client {
        conn_id: ConnId,
        msg: ClientMsg,
    },
    Disconnected {
        conn_id: ConnId,
    },
}

struct SeatSlot {
    name: String,
    difficulty: Option<Difficulty>,
    /// Session token a human uses to reclaim this seat after a disconnect.
    token: Option<String>,
    conn_id: Option<ConnId>,
    conn: Option<mpsc::Sender<ServerMsg>>,
    /// Lobby only: when a disconnected seat is freed if its player never
    /// rejoins.
    vacate_at: Option<Instant>,
    last_emote: Option<Instant>,
}

impl SeatSlot {
    fn is_bot(&self) -> bool {
        self.difficulty.is_some()
    }
}

pub fn spawn_room(registry: Arc<Registry>) -> (String, mpsc::Sender<RoomCmd>) {
    let (tx, rx) = mpsc::channel(64);
    let code = registry.create(tx.clone());
    let room = Room {
        code: code.clone(),
        seats: Vec::new(),
        game: None,
        bots: Vec::new(),
        rng: StdRng::from_os_rng(),
        rx,
        next_conn_id: 1,
        bot_deadline: None,
        gc_deadline: Some(Instant::now() + gc_timeout()),
    };
    let task_code = code.clone();
    tokio::spawn(async move {
        // The registry entry must go even if the room panics; a leaked entry
        // is a room code that accepts joins nobody will ever answer.
        if AssertUnwindSafe(room.run()).catch_unwind().await.is_err() {
            tracing::error!(code = %task_code, "room task panicked");
        }
        registry.remove(&task_code);
        tracing::info!(code = %task_code, "room closed");
    });
    (code, tx)
}

fn gc_timeout() -> Duration {
    std::env::var("GOLF_ROOM_GC_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or(DEFAULT_GC)
}

fn lobby_grace() -> Duration {
    std::env::var("GOLF_LOBBY_GRACE_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or(DEFAULT_LOBBY_GRACE)
}

struct Room {
    code: String,
    seats: Vec<SeatSlot>,
    game: Option<MatchState>,
    bots: Vec<Option<Box<dyn Bot>>>,
    rng: StdRng,
    rx: mpsc::Receiver<RoomCmd>,
    next_conn_id: ConnId,
    bot_deadline: Option<Instant>,
    gc_deadline: Option<Instant>,
}

impl Room {
    async fn run(mut self) {
        loop {
            let cmd = if let Some(deadline) = self.next_deadline() {
                tokio::select! {
                    cmd = self.rx.recv() => match cmd {
                        Some(cmd) => Some(cmd),
                        None => break,
                    },
                    _ = tokio::time::sleep_until(deadline) => None,
                }
            } else {
                match self.rx.recv().await {
                    Some(cmd) => Some(cmd),
                    None => break,
                }
            };
            match cmd {
                Some(cmd) => self.handle(cmd).await,
                None => {
                    if self.fire_deadlines().await {
                        break; // garbage-collected
                    }
                }
            }
        }
    }

    fn next_deadline(&self) -> Option<Instant> {
        let vacate = self.seats.iter().filter_map(|s| s.vacate_at).min();
        [self.bot_deadline, self.gc_deadline, vacate]
            .into_iter()
            .flatten()
            .min()
    }

    /// Returns true when the room should shut down.
    async fn fire_deadlines(&mut self) -> bool {
        let now = Instant::now();
        if self.gc_deadline.is_some_and(|d| d <= now) {
            if self.seats.iter().all(|s| s.conn.is_none()) {
                return true;
            }
            self.gc_deadline = None;
        }
        if self.bot_deadline.is_some_and(|d| d <= now) {
            self.bot_deadline = None;
            self.bot_step().await;
        }
        // Free lobby seats whose disconnect grace ran out.
        if !self.started() {
            let before = self.seats.len();
            self.seats.retain(|s| s.vacate_at.is_none_or(|d| d > now));
            if self.seats.len() != before {
                self.broadcast_lobby().await;
            }
        }
        false
    }

    async fn handle(&mut self, cmd: RoomCmd) {
        match cmd {
            RoomCmd::Join { name, conn, reply } => {
                let result = self.join(name, conn).await;
                let _ = reply.send(result);
            }
            RoomCmd::Rejoin { token, conn, reply } => {
                let result = self.rejoin(token, conn).await;
                let _ = reply.send(result);
            }
            RoomCmd::Client { conn_id, msg } => self.client_msg(conn_id, msg).await,
            RoomCmd::Disconnected { conn_id } => self.disconnected(conn_id).await,
        }
    }

    async fn join(
        &mut self,
        name: String,
        conn: mpsc::Sender<ServerMsg>,
    ) -> Result<ConnId, ErrorCode> {
        if self.started() {
            return Err(ErrorCode::MatchAlreadyStarted);
        }
        if self.seats.len() >= MAX_SEATS {
            return Err(ErrorCode::RoomFull);
        }
        let conn_id = self.next_conn_id;
        self.next_conn_id += 1;
        let token = uuid::Uuid::new_v4().to_string();
        let seat = self.seats.len() as Seat;
        self.seats.push(SeatSlot {
            name: sanitize_name(&name, seat),
            difficulty: None,
            token: Some(token.clone()),
            conn_id: Some(conn_id),
            conn: Some(conn.clone()),
            vacate_at: None,
            last_emote: None,
        });
        self.gc_deadline = None;
        let _ = conn
            .send(ServerMsg::RoomJoined {
                code: self.code.clone(),
                seat,
                token,
            })
            .await;
        self.broadcast_lobby().await;
        Ok(conn_id)
    }

    async fn rejoin(
        &mut self,
        token: String,
        conn: mpsc::Sender<ServerMsg>,
    ) -> Result<ConnId, ErrorCode> {
        let Some(seat) = self
            .seats
            .iter()
            .position(|s| s.token.as_deref() == Some(token.as_str()))
        else {
            return Err(ErrorCode::BadToken);
        };
        let conn_id = self.next_conn_id;
        self.next_conn_id += 1;
        let slot = &mut self.seats[seat];
        slot.conn_id = Some(conn_id);
        slot.conn = Some(conn.clone());
        slot.vacate_at = None;
        self.gc_deadline = None;
        let _ = conn
            .send(ServerMsg::RoomJoined {
                code: self.code.clone(),
                seat: seat as Seat,
                token,
            })
            .await;
        let _ = conn
            .send(ServerMsg::Lobby {
                state: self.lobby_state(seat as Seat),
            })
            .await;
        if let Some(game) = &self.game {
            let _ = conn
                .send(ServerMsg::State {
                    view: game.view(Viewer::Seat(seat as Seat)),
                })
                .await;
        }
        self.broadcast_lobby().await;
        Ok(conn_id)
    }

    async fn client_msg(&mut self, conn_id: ConnId, msg: ClientMsg) {
        let Some(seat) = self.seat_of(conn_id) else {
            return;
        };
        match msg {
            ClientMsg::Ping => self.send_to(seat, ServerMsg::Pong).await,
            ClientMsg::AddBot { difficulty } => {
                if let Err(code) = self.add_bot(seat, difficulty) {
                    self.send_error(seat, code).await;
                } else {
                    self.broadcast_lobby().await;
                }
            }
            ClientMsg::RemoveSeat { seat: target } => {
                if let Err(code) = self.remove_seat(seat, target).await {
                    self.send_error(seat, code).await;
                } else {
                    self.broadcast_lobby().await;
                }
            }
            ClientMsg::Emote { emote } => self.emote(seat, emote).await,
            ClientMsg::StartMatch { holes } => match self.start_match(seat, holes) {
                Err(code) => self.send_error(seat, code).await,
                Ok(events) => {
                    self.broadcast_lobby().await;
                    self.broadcast_game(&events).await;
                    self.schedule_bots();
                }
            },
            ClientMsg::Act { action } => {
                let Some(game) = &mut self.game else {
                    self.send_error(seat, ErrorCode::MatchNotStarted).await;
                    return;
                };
                match game.apply(seat, action) {
                    Err(err) => {
                        let _ = self.seats[seat as usize]
                            .conn
                            .as_ref()
                            .unwrap()
                            .send(ServerMsg::Error {
                                code: ErrorCode::BadAction,
                                message: err.to_string(),
                            })
                            .await;
                    }
                    Ok(events) => {
                        self.broadcast_game(&events).await;
                        self.schedule_bots();
                    }
                }
            }
            // Join/Rejoin/Create are connection-level; ignore here.
            ClientMsg::CreateRoom { .. }
            | ClientMsg::JoinRoom { .. }
            | ClientMsg::Rejoin { .. } => {
                self.send_error(seat, ErrorCode::BadRequest).await;
            }
        }
    }

    fn add_bot(&mut self, seat: Seat, difficulty: Difficulty) -> Result<(), ErrorCode> {
        self.require_host(seat)?;
        if self.started() {
            return Err(ErrorCode::MatchAlreadyStarted);
        }
        if self.seats.len() >= MAX_SEATS {
            return Err(ErrorCode::RoomFull);
        }
        let count = self.seats.iter().filter(|s| s.is_bot()).count();
        self.seats.push(SeatSlot {
            name: format!("{difficulty:?} bot {}", count + 1),
            difficulty: Some(difficulty),
            token: None,
            conn_id: None,
            conn: None,
            vacate_at: None,
            last_emote: None,
        });
        Ok(())
    }

    /// Host removes a seat. In the lobby the seat simply goes away; mid-game
    /// a human seat is handed to a bot so the match keeps moving (the usual
    /// reason to kick: someone left and won't be back).
    async fn remove_seat(&mut self, seat: Seat, target: Seat) -> Result<(), ErrorCode> {
        self.require_host(seat)?;
        if target == seat {
            return Err(ErrorCode::BadRequest);
        }
        if self.seats.get(target as usize).is_none() {
            return Err(ErrorCode::BadRequest);
        }
        if self.started() {
            if self.seats[target as usize].is_bot() {
                return Err(ErrorCode::BadRequest);
            }
            self.send_error(target, ErrorCode::Kicked).await;
            let slot = &mut self.seats[target as usize];
            slot.conn = None;
            slot.conn_id = None;
            slot.token = None;
            slot.difficulty = Some(TAKEOVER_BOT);
            self.bots[target as usize] = Some(make_bot(TAKEOVER_BOT));
            self.schedule_bots();
        } else {
            self.send_error(target, ErrorCode::Kicked).await;
            self.seats.remove(target as usize);
        }
        Ok(())
    }

    /// Fan a reaction out to the table, dropping spam.
    async fn emote(&mut self, seat: Seat, emote: Emote) {
        let now = Instant::now();
        let slot = &mut self.seats[seat as usize];
        if slot.last_emote.is_some_and(|t| now - t < EMOTE_COOLDOWN) {
            return;
        }
        slot.last_emote = Some(now);
        for slot in &self.seats {
            if let Some(conn) = &slot.conn {
                let _ = conn.send(ServerMsg::Emote { seat, emote }).await;
            }
        }
    }

    fn start_match(&mut self, seat: Seat, holes: u8) -> Result<Vec<Event>, ErrorCode> {
        self.require_host(seat)?;
        if self.started() {
            return Err(ErrorCode::MatchAlreadyStarted);
        }
        if self.seats.len() < 2 {
            return Err(ErrorCode::BadRequest);
        }
        let config = MatchConfig {
            seats: self
                .seats
                .iter()
                .map(|s| SeatConfig {
                    name: s.name.clone(),
                    kind: match s.difficulty {
                        Some(difficulty) => SeatKind::Bot { difficulty },
                        None => SeatKind::Human,
                    },
                })
                .collect(),
            holes: holes.clamp(1, 18),
        };
        let game = MatchState::new(config, self.rng.random()).map_err(|_| ErrorCode::BadRequest)?;
        // Seats are held for the whole match; cancel any pending lobby reaps.
        for slot in &mut self.seats {
            slot.vacate_at = None;
        }
        self.bots = self
            .seats
            .iter()
            .map(|s| s.difficulty.map(make_bot))
            .collect();
        let events = vec![
            Event::HoleDealt {
                hole: 1,
                starting_seat: 0,
                discard_start: *game.hole.discard.last().expect("discard seeded at deal"),
            },
            Event::TurnStarted { seat: 0 },
        ];
        self.game = Some(game);
        Ok(events)
    }

    async fn bot_step(&mut self) {
        let Some(game) = &mut self.game else {
            return;
        };
        let Some(seat) = game.seat_to_act() else {
            return;
        };
        let Some(Some(bot)) = self.bots.get_mut(seat as usize) else {
            return;
        };
        let view = game.view(Viewer::Seat(seat));
        let action = bot.choose(&view, &mut self.rng);
        match game.apply(seat, action) {
            Ok(events) => {
                self.broadcast_game(&events).await;
                self.schedule_bots();
            }
            Err(err) => {
                // Bots are tested to only produce legal actions.
                tracing::error!(code = %self.code, seat, %err, "bot chose an illegal action");
            }
        }
    }

    fn schedule_bots(&mut self) {
        let bot_up = self
            .game
            .as_ref()
            .and_then(|g| g.seat_to_act())
            .is_some_and(|s| self.bots.get(s as usize).is_some_and(|b| b.is_some()));
        self.bot_deadline = bot_up.then(|| Instant::now() + bot_step_delay());
    }

    async fn disconnected(&mut self, conn_id: ConnId) {
        let Some(seat) = self.seat_of(conn_id) else {
            return;
        };
        let started = self.started();
        let slot = &mut self.seats[seat as usize];
        slot.conn = None;
        slot.conn_id = None;
        // In the lobby the seat is held for a grace period so a network blip
        // doesn't cost the player their place (the host's seat is always
        // held — it keeps the room alive to return to).
        if !started && seat != 0 {
            slot.vacate_at = Some(Instant::now() + lobby_grace());
        }
        if self.seats.iter().all(|s| s.conn.is_none()) {
            self.gc_deadline = Some(Instant::now() + gc_timeout());
        }
        self.broadcast_lobby().await;
    }

    fn started(&self) -> bool {
        self.game
            .as_ref()
            .is_some_and(|g| !matches!(g.phase, Phase::MatchComplete { .. }))
    }

    fn require_host(&self, seat: Seat) -> Result<(), ErrorCode> {
        if seat == 0 {
            Ok(())
        } else {
            Err(ErrorCode::NotHost)
        }
    }

    fn seat_of(&self, conn_id: ConnId) -> Option<Seat> {
        self.seats
            .iter()
            .position(|s| s.conn_id == Some(conn_id))
            .map(|i| i as Seat)
    }

    fn lobby_state(&self, you: Seat) -> LobbyState {
        LobbyState {
            code: self.code.clone(),
            seats: self
                .seats
                .iter()
                .map(|s| LobbySeat {
                    name: s.name.clone(),
                    difficulty: s.difficulty,
                    connected: s.conn.is_some() || s.is_bot(),
                })
                .collect(),
            host: 0,
            you,
            started: self.started(),
        }
    }

    async fn broadcast_lobby(&self) {
        for (i, slot) in self.seats.iter().enumerate() {
            if let Some(conn) = &slot.conn {
                let _ = conn
                    .send(ServerMsg::Lobby {
                        state: self.lobby_state(i as Seat),
                    })
                    .await;
            }
        }
    }

    async fn broadcast_game(&self, events: &[Event]) {
        let Some(game) = &self.game else {
            return;
        };
        for (i, slot) in self.seats.iter().enumerate() {
            if let Some(conn) = &slot.conn {
                if !events.is_empty() {
                    let _ = conn
                        .send(ServerMsg::Events {
                            events: events.to_vec(),
                        })
                        .await;
                }
                let _ = conn
                    .send(ServerMsg::State {
                        view: game.view(Viewer::Seat(i as Seat)),
                    })
                    .await;
            }
        }
    }

    async fn send_to(&self, seat: Seat, msg: ServerMsg) {
        if let Some(conn) = self.seats[seat as usize].conn.as_ref() {
            let _ = conn.send(msg).await;
        }
    }

    async fn send_error(&self, seat: Seat, code: ErrorCode) {
        self.send_to(
            seat,
            ServerMsg::Error {
                code,
                message: format!("{code:?}"),
            },
        )
        .await;
    }
}

fn sanitize_name(name: &str, seat: Seat) -> String {
    let trimmed: String = name.trim().chars().take(14).collect();
    if trimmed.is_empty() {
        format!("Player {}", seat + 1)
    } else {
        trimmed
    }
}
