use crate::protocol::{ClientMsg, ErrorCode, ServerMsg};
use crate::room::{spawn_room, ConnId, RoomCmd};
use crate::AppState;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

pub async fn ws_handler(ws: WebSocketUpgrade, State(app): State<Arc<AppState>>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, app))
}

async fn handle_socket(socket: WebSocket, app: Arc<AppState>) {
    let (mut sink, mut stream) = socket.split();
    // Room → socket forwarding runs in its own task so the room never blocks
    // on a slow client.
    let (tx, mut rx) = mpsc::channel::<ServerMsg>(64);
    let forward = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let text = serde_json::to_string(&msg).expect("ServerMsg serializes");
            if sink.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    let mut room: Option<(mpsc::Sender<RoomCmd>, ConnId)> = None;

    while let Some(Ok(msg)) = stream.next().await {
        let Message::Text(text) = msg else {
            if matches!(msg, Message::Close(_)) {
                break;
            }
            continue;
        };
        let Ok(client_msg) = serde_json::from_str::<ClientMsg>(&text) else {
            send_error(&tx, ErrorCode::BadRequest, "unrecognized message").await;
            continue;
        };

        match (&room, client_msg) {
            // Already seated: everything else goes to the room actor.
            (Some((room_tx, conn_id)), msg) => {
                if room_tx
                    .send(RoomCmd::Client {
                        conn_id: *conn_id,
                        msg,
                    })
                    .await
                    .is_err()
                {
                    break; // room is gone
                }
            }
            (None, ClientMsg::CreateRoom { name }) => {
                let (_, room_tx) = spawn_room(app.registry.clone());
                room = seat_in_room(room_tx, RoomJoinKind::Join(name), &tx).await;
            }
            (None, ClientMsg::JoinRoom { code, name }) => {
                match app.registry.get(&code.to_uppercase()) {
                    Some(room_tx) => {
                        room = seat_in_room(room_tx, RoomJoinKind::Join(name), &tx).await;
                    }
                    None => send_error(&tx, ErrorCode::RoomNotFound, "no such table").await,
                }
            }
            (None, ClientMsg::Rejoin { code, token }) => {
                match app.registry.get(&code.to_uppercase()) {
                    Some(room_tx) => {
                        room = seat_in_room(room_tx, RoomJoinKind::Rejoin(token), &tx).await;
                    }
                    None => send_error(&tx, ErrorCode::RoomNotFound, "no such table").await,
                }
            }
            (None, _) => send_error(&tx, ErrorCode::BadRequest, "join a table first").await,
        }
    }

    if let Some((room_tx, conn_id)) = room {
        let _ = room_tx.send(RoomCmd::Disconnected { conn_id }).await;
    }
    forward.abort();
}

enum RoomJoinKind {
    Join(String),
    Rejoin(String),
}

async fn seat_in_room(
    room_tx: mpsc::Sender<RoomCmd>,
    kind: RoomJoinKind,
    conn: &mpsc::Sender<ServerMsg>,
) -> Option<(mpsc::Sender<RoomCmd>, ConnId)> {
    let (reply, reply_rx) = oneshot::channel();
    let cmd = match kind {
        RoomJoinKind::Join(name) => RoomCmd::Join {
            name,
            conn: conn.clone(),
            reply,
        },
        RoomJoinKind::Rejoin(token) => RoomCmd::Rejoin {
            token,
            conn: conn.clone(),
            reply,
        },
    };
    if room_tx.send(cmd).await.is_err() {
        send_error(conn, ErrorCode::RoomNotFound, "table just closed").await;
        return None;
    }
    match reply_rx.await {
        Ok(Ok(conn_id)) => Some((room_tx, conn_id)),
        Ok(Err(code)) => {
            send_error(conn, code, "could not join").await;
            None
        }
        Err(_) => None,
    }
}

async fn send_error(conn: &mpsc::Sender<ServerMsg>, code: ErrorCode, message: &str) {
    let _ = conn
        .send(ServerMsg::Error {
            code,
            message: message.to_string(),
        })
        .await;
}
