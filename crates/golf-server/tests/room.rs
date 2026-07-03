//! End-to-end server test: two WebSocket clients create/join a room, add a
//! bot, play a full hole (bot turns run automatically), verify per-client
//! redaction, and exercise disconnect + rejoin.

use futures_util::{SinkExt, StreamExt};
use golf_engine::{PhaseView, SlotView};
use golf_server::protocol::{ClientMsg, ServerMsg};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

struct Client {
    tx: futures_util::stream::SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    rx: mpsc::UnboundedReceiver<ServerMsg>,
    reader: tokio::task::JoinHandle<()>,
}

impl Drop for Client {
    fn drop(&mut self) {
        // The reader task owns the stream half of the socket; killing it is
        // what actually closes the TCP connection so the server notices.
        self.reader.abort();
    }
}

impl Client {
    async fn connect(addr: &str) -> Client {
        let (ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws"))
            .await
            .expect("connect");
        let (tx, mut stream) = ws.split();
        let (msg_tx, rx) = mpsc::unbounded_channel();
        let reader = tokio::spawn(async move {
            while let Some(Ok(msg)) = stream.next().await {
                if let Message::Text(text) = msg {
                    let parsed: ServerMsg = serde_json::from_str(&text).expect("valid ServerMsg");
                    if msg_tx.send(parsed).is_err() {
                        break;
                    }
                }
            }
        });
        Client { tx, rx, reader }
    }

    async fn send(&mut self, msg: &ClientMsg) {
        self.tx
            .send(Message::Text(serde_json::to_string(msg).unwrap().into()))
            .await
            .expect("send");
    }

    async fn recv(&mut self) -> ServerMsg {
        tokio::time::timeout(Duration::from_secs(10), self.rx.recv())
            .await
            .expect("timed out waiting for a server message")
            .expect("connection closed")
    }

    /// Read messages until the predicate extracts a value.
    async fn recv_until<T>(&mut self, mut f: impl FnMut(ServerMsg) -> Option<T>) -> T {
        loop {
            let msg = self.recv().await;
            if let Some(value) = f(msg) {
                return value;
            }
        }
    }

    async fn next_state(&mut self) -> golf_engine::PlayerView {
        self.recv_until(|m| match m {
            ServerMsg::State { view } => Some(view),
            _ => None,
        })
        .await
    }
}

async fn start_server() -> String {
    // Fast bots, quick GC for the test process.
    std::env::set_var("GOLF_BOT_STEP_MS", "5");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, golf_server::app()).await.unwrap();
    });
    addr.to_string()
}

/// Assert the redaction contract from `viewer`'s perspective.
fn assert_redacted(view: &golf_engine::PlayerView, viewer: u8) {
    assert_eq!(view.viewer, Some(viewer));
    for (owner, seat) in view.seats.iter().enumerate() {
        for slot in &seat.grid {
            if owner as u8 != viewer {
                assert!(
                    !matches!(slot, SlotView::Peeked { .. }),
                    "seat {viewer} sees a peeked card of seat {owner}"
                );
            }
        }
    }
}

#[tokio::test]
async fn full_online_flow() {
    let addr = start_server().await;

    // Ann opens a table.
    let mut ann = Client::connect(&addr).await;
    ann.send(&ClientMsg::CreateRoom { name: "Ann".into() })
        .await;
    let (code, ann_token) = ann
        .recv_until(|m| match m {
            ServerMsg::RoomJoined { code, seat, token } => {
                assert_eq!(seat, 0);
                Some((code, token))
            }
            _ => None,
        })
        .await;
    assert_eq!(code.len(), 4);
    let _ = ann_token;

    // Ben joins by code (lowercase must work too).
    let mut ben = Client::connect(&addr).await;
    ben.send(&ClientMsg::JoinRoom {
        code: code.to_lowercase(),
        name: "Ben".into(),
    })
    .await;
    let ben_token = ben
        .recv_until(|m| match m {
            ServerMsg::RoomJoined { seat, token, .. } => {
                assert_eq!(seat, 1);
                Some(token)
            }
            _ => None,
        })
        .await;

    // Ann sees Ben arrive, adds a bot, and starts a 1-hole match.
    ann.recv_until(|m| match m {
        ServerMsg::Lobby { state } if state.seats.len() == 2 => Some(()),
        _ => None,
    })
    .await;
    ann.send(&ClientMsg::AddBot {
        difficulty: golf_engine::bot::Difficulty::Easy,
    })
    .await;
    ann.send(&ClientMsg::StartMatch { holes: 1 }).await;

    // Ben must NOT be able to start or add bots (not host).
    ben.send(&ClientMsg::AddBot {
        difficulty: golf_engine::bot::Difficulty::Easy,
    })
    .await;
    ben.recv_until(|m| match m {
        ServerMsg::Error { code, .. } => {
            assert!(matches!(
                code,
                golf_server::protocol::ErrorCode::NotHost
                    | golf_server::protocol::ErrorCode::MatchAlreadyStarted
            ));
            Some(())
        }
        _ => None,
    })
    .await;

    // Play the hole out. Each client acts when its seat is current.
    let mut ann_view = ann.next_state().await;
    let mut ben_view = ben.next_state().await;
    let mut ben_states = 1;
    loop {
        assert_redacted(&ann_view, 0);
        assert_redacted(&ben_view, 1);
        if let PhaseView::HoleComplete { scores, .. } = &ann_view.phase {
            assert_eq!(scores.len(), 3);
            break;
        }
        // Drive whichever human is up; the bot moves on its own.
        if ann_view.current == 0 && !ann_view.legal_actions.is_empty() {
            let action = next_action(&ann_view);
            ann.send(&ClientMsg::Act { action }).await;
        } else if ben_view.current == 1 && !ben_view.legal_actions.is_empty() {
            let action = next_action(&ben_view);
            ben.send(&ClientMsg::Act { action }).await;
        }
        // Both clients receive a fresh snapshot after every mutation.
        ann_view = ann.next_state().await;
        ben_view = ben.next_state().await;
        ben_states += 1;
        assert!(ben_states < 500, "hole did not complete");
    }

    // Ben drops and rejoins with his token; his private view is restored.
    let ben_peeked_before: Vec<_> = ben_view.seats[1]
        .grid
        .iter()
        .filter(|s| matches!(s, SlotView::Peeked { .. } | SlotView::FaceUp { .. }))
        .cloned()
        .collect();
    drop(ben);
    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut ben2 = Client::connect(&addr).await;
    ben2.send(&ClientMsg::Rejoin {
        code: code.clone(),
        token: ben_token,
    })
    .await;
    ben2.recv_until(|m| match m {
        ServerMsg::RoomJoined { seat, .. } => {
            assert_eq!(seat, 1);
            Some(())
        }
        _ => None,
    })
    .await;
    let restored = ben2.next_state().await;
    assert_redacted(&restored, 1);
    let ben_after: Vec<_> = restored.seats[1]
        .grid
        .iter()
        .filter(|s| matches!(s, SlotView::Peeked { .. } | SlotView::FaceUp { .. }))
        .cloned()
        .collect();
    assert_eq!(
        ben_peeked_before, ben_after,
        "rejoin lost private knowledge"
    );

    // A bad token is rejected.
    let mut crook = Client::connect(&addr).await;
    crook
        .send(&ClientMsg::Rejoin {
            code: code.clone(),
            token: "not-a-token".into(),
        })
        .await;
    crook
        .recv_until(|m| match m {
            ServerMsg::Error { code, .. } => {
                assert_eq!(code, golf_server::protocol::ErrorCode::BadToken);
                Some(())
            }
            _ => None,
        })
        .await;

    // Joining a nonexistent room fails cleanly.
    let mut lost = Client::connect(&addr).await;
    lost.send(&ClientMsg::JoinRoom {
        code: "ZZZZ".into(),
        name: "Zoe".into(),
    })
    .await;
    lost.recv_until(|m| match m {
        ServerMsg::Error { code, .. } => {
            assert_eq!(code, golf_server::protocol::ErrorCode::RoomNotFound);
            Some(())
        }
        _ => None,
    })
    .await;
}

/// A lobby disconnect must NOT free the seat immediately: the player gets a
/// grace period to rejoin with their token (network blips are routine).
#[tokio::test]
async fn lobby_disconnect_grace_allows_rejoin() {
    let addr = start_server().await;

    let mut ann = Client::connect(&addr).await;
    ann.send(&ClientMsg::CreateRoom { name: "Ann".into() })
        .await;
    let code = ann
        .recv_until(|m| match m {
            ServerMsg::RoomJoined { code, .. } => Some(code),
            _ => None,
        })
        .await;

    let mut ben = Client::connect(&addr).await;
    ben.send(&ClientMsg::JoinRoom {
        code: code.clone(),
        name: "Ben".into(),
    })
    .await;
    let ben_token = ben
        .recv_until(|m| match m {
            ServerMsg::RoomJoined { token, .. } => Some(token),
            _ => None,
        })
        .await;

    // Ben's connection blips. Ann must still see his (offline) seat.
    drop(ben);
    ann.recv_until(|m| match m {
        ServerMsg::Lobby { state } if state.seats.len() == 2 && !state.seats[1].connected => {
            Some(())
        }
        _ => None,
    })
    .await;

    // Within the grace period the token reclaims the seat.
    let mut ben2 = Client::connect(&addr).await;
    ben2.send(&ClientMsg::Rejoin {
        code: code.clone(),
        token: ben_token,
    })
    .await;
    ben2.recv_until(|m| match m {
        ServerMsg::RoomJoined { seat, .. } => {
            assert_eq!(seat, 1);
            Some(())
        }
        _ => None,
    })
    .await;
}

/// The host can kick anyone from the lobby; the kicked player is told and
/// their token stops working.
#[tokio::test]
async fn host_kicks_player_from_lobby() {
    let addr = start_server().await;

    let mut ann = Client::connect(&addr).await;
    ann.send(&ClientMsg::CreateRoom { name: "Ann".into() })
        .await;
    let code = ann
        .recv_until(|m| match m {
            ServerMsg::RoomJoined { code, .. } => Some(code),
            _ => None,
        })
        .await;

    let mut ben = Client::connect(&addr).await;
    ben.send(&ClientMsg::JoinRoom {
        code: code.clone(),
        name: "Ben".into(),
    })
    .await;
    let ben_token = ben
        .recv_until(|m| match m {
            ServerMsg::RoomJoined { token, .. } => Some(token),
            _ => None,
        })
        .await;

    // Ben must not be able to kick (not host).
    ben.send(&ClientMsg::RemoveSeat { seat: 0 }).await;
    ben.recv_until(|m| match m {
        ServerMsg::Error { code, .. } => {
            assert_eq!(code, golf_server::protocol::ErrorCode::NotHost);
            Some(())
        }
        _ => None,
    })
    .await;

    ann.send(&ClientMsg::RemoveSeat { seat: 1 }).await;
    ben.recv_until(|m| match m {
        ServerMsg::Error { code, .. } => {
            assert_eq!(code, golf_server::protocol::ErrorCode::Kicked);
            Some(())
        }
        _ => None,
    })
    .await;
    ann.recv_until(|m| match m {
        ServerMsg::Lobby { state } if state.seats.len() == 1 => Some(()),
        _ => None,
    })
    .await;

    // The kicked token is dead.
    let mut ben2 = Client::connect(&addr).await;
    ben2.send(&ClientMsg::Rejoin {
        code,
        token: ben_token,
    })
    .await;
    ben2.recv_until(|m| match m {
        ServerMsg::Error { code, .. } => {
            assert_eq!(code, golf_server::protocol::ErrorCode::BadToken);
            Some(())
        }
        _ => None,
    })
    .await;
}

/// Kicking a human mid-game hands their seat to a bot so the match keeps
/// moving — the fix for a table stalled on someone who left for good.
#[tokio::test]
async fn mid_game_kick_hands_seat_to_bot() {
    let addr = start_server().await;

    let mut ann = Client::connect(&addr).await;
    ann.send(&ClientMsg::CreateRoom { name: "Ann".into() })
        .await;
    let code = ann
        .recv_until(|m| match m {
            ServerMsg::RoomJoined { code, .. } => Some(code),
            _ => None,
        })
        .await;

    let mut ben = Client::connect(&addr).await;
    ben.send(&ClientMsg::JoinRoom {
        code: code.clone(),
        name: "Ben".into(),
    })
    .await;
    ben.recv_until(|m| match m {
        ServerMsg::RoomJoined { .. } => Some(()),
        _ => None,
    })
    .await;

    ann.recv_until(|m| match m {
        ServerMsg::Lobby { state } if state.seats.len() == 2 => Some(()),
        _ => None,
    })
    .await;
    ann.send(&ClientMsg::StartMatch { holes: 1 }).await;
    let mut view = ann.next_state().await;

    // Kick Ben; he's told, and his seat plays on as a bot.
    ann.send(&ClientMsg::RemoveSeat { seat: 1 }).await;
    ben.recv_until(|m| match m {
        ServerMsg::Error { code, .. } => {
            assert_eq!(code, golf_server::protocol::ErrorCode::Kicked);
            Some(())
        }
        _ => None,
    })
    .await;

    // Ann alone can finish the hole: the takeover bot moves for seat 1.
    let mut n = 0;
    loop {
        if matches!(view.phase, PhaseView::HoleComplete { .. }) {
            break;
        }
        if view.current == 0 && !view.legal_actions.is_empty() {
            let action = next_action(&view);
            ann.send(&ClientMsg::Act { action }).await;
        }
        view = ann.next_state().await;
        n += 1;
        assert!(n < 500, "hole did not complete after the kick");
    }
}

/// Emotes fan out to the whole table, and spam inside the cooldown window is
/// silently dropped.
#[tokio::test]
async fn emotes_broadcast_and_rate_limit() {
    let addr = start_server().await;

    let mut ann = Client::connect(&addr).await;
    ann.send(&ClientMsg::CreateRoom { name: "Ann".into() })
        .await;
    let code = ann
        .recv_until(|m| match m {
            ServerMsg::RoomJoined { code, .. } => Some(code),
            _ => None,
        })
        .await;

    let mut ben = Client::connect(&addr).await;
    ben.send(&ClientMsg::JoinRoom {
        code: code.clone(),
        name: "Ben".into(),
    })
    .await;
    ben.recv_until(|m| match m {
        ServerMsg::RoomJoined { .. } => Some(()),
        _ => None,
    })
    .await;

    // Two back-to-back emotes: only the first may land. The Ping/Pong pair
    // brackets the burst so we know the room processed all of it.
    ben.send(&ClientMsg::Emote {
        emote: golf_server::protocol::Emote::Fire,
    })
    .await;
    ben.send(&ClientMsg::Emote {
        emote: golf_server::protocol::Emote::Cry,
    })
    .await;
    ben.send(&ClientMsg::Ping).await;

    let mut seen = Vec::new();
    ben.recv_until(|m| match m {
        ServerMsg::Emote { seat, emote } => {
            seen.push((seat, emote));
            None
        }
        ServerMsg::Pong => Some(()),
        _ => None,
    })
    .await;
    assert_eq!(seen, vec![(1, golf_server::protocol::Emote::Fire)]);

    // Everyone at the table sees it, not just the sender.
    ann.recv_until(|m| match m {
        ServerMsg::Emote { seat, emote } => {
            assert_eq!(seat, 1);
            assert_eq!(emote, golf_server::protocol::Emote::Fire);
            Some(())
        }
        _ => None,
    })
    .await;
}

/// Pick a simple legal action from a view (draw → flip first face-down → keep).
fn next_action(view: &golf_engine::PlayerView) -> golf_engine::Action {
    use golf_engine::{Action, TurnState};
    let me = view.viewer.unwrap() as usize;
    match view.turn {
        TurnState::AwaitDraw => Action::DrawFromDeck,
        TurnState::AwaitFlip { .. } => {
            let slot = view.seats[me]
                .grid
                .iter()
                .position(|s| !matches!(s, SlotView::FaceUp { .. }))
                .expect("a face-down slot exists") as u8;
            Action::Flip { slot }
        }
        TurnState::AwaitResolve { .. } => Action::Keep,
    }
}
