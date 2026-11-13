//! One WebSocket connection: the handshake, then the session loop.

use std::time::Duration;

use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt, stream::SplitSink};
use malmoon_auth::CurrentSession;
use malmoon_core::protocol::{ClientMessage, ErrorCode, PROTOCOL_VERSION, ServerMessage};
use tokio::sync::mpsc;

use crate::{AppState, demo};

/// How long a new connection may take to send `hello`.
const HELLO_TIMEOUT: Duration = Duration::from_secs(10);

/// Only signed-in clients get a session: without one the handshake is refused with 401 before
/// it becomes a WebSocket.
pub(crate) async fn upgrade(
    account: CurrentSession,
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| run(socket, state, account))
}

async fn run(socket: WebSocket, state: AppState, account: CurrentSession) {
    let (mut sink, mut stream) = socket.split();

    let hello = tokio::time::timeout(HELLO_TIMEOUT, stream.next()).await;
    let client = match hello {
        Ok(Some(Ok(Message::Text(text)))) => match serde_json::from_str(&text) {
            Ok(ClientMessage::Hello {
                protocol_version: PROTOCOL_VERSION,
                client,
            }) => client,
            Ok(ClientMessage::Hello {
                protocol_version, ..
            }) => {
                let message = format!(
                    "this gateway speaks protocol {PROTOCOL_VERSION}, the client sent {protocol_version}"
                );
                return reject(sink, ErrorCode::UnsupportedProtocol, message).await;
            }
            Err(error) => {
                return reject(sink, ErrorCode::ExpectedHello, error.to_string()).await;
            }
        },
        // The client went away before saying anything. Nobody is left to tell.
        Ok(None | Some(Err(_) | Ok(Message::Close(_)))) => {
            tracing::debug!("connection closed before hello");
            return;
        }
        Ok(Some(Ok(_))) | Err(_) => {
            let message = "the first message must be a hello text frame".to_owned();
            return reject(sink, ErrorCode::ExpectedHello, message).await;
        }
    };

    let session_id = state.next_session_id();
    tracing::info!(%session_id, ?client, user_id = %account.user_id, "session opened");
    let ready = ServerMessage::Ready {
        protocol_version: PROTOCOL_VERSION,
        session_id: session_id.clone(),
    };
    if send(&mut sink, &ready).await.is_err() {
        return;
    }

    // Everything sent to the client after `ready` goes through this channel, so that several
    // producers (today only the demo feed) can share the socket.
    let (outbound, mut outbox) = mpsc::channel::<ServerMessage>(64);
    let demo_feed = state
        .config
        .demo_captions
        .then(|| tokio::spawn(demo::run(outbound.clone())));
    drop(outbound);

    let mut audio_bytes = 0usize;
    loop {
        tokio::select! {
            Some(message) = outbox.recv() => {
                if send(&mut sink, &message).await.is_err() {
                    break;
                }
            }
            frame = stream.next() => match frame {
                // Audio. No speech model is connected yet, so the frames are only counted.
                Some(Ok(Message::Binary(bytes))) => audio_bytes += bytes.len(),
                Some(Ok(Message::Text(text))) => {
                    if let Err(error) = serde_json::from_str::<ClientMessage>(&text) {
                        let message = ServerMessage::Error {
                            code: ErrorCode::InvalidMessage,
                            message: error.to_string(),
                        };
                        let _ = send(&mut sink, &message).await;
                        break;
                    }
                }
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                // axum answers pings itself.
                Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
            },
        }
    }

    if let Some(feed) = demo_feed {
        feed.abort();
    }
    tracing::info!(%session_id, audio_bytes, "session closed");
}

async fn reject(mut sink: SplitSink<WebSocket, Message>, code: ErrorCode, message: String) {
    tracing::warn!(?code, %message, "session rejected");
    let _ = send(&mut sink, &ServerMessage::Error { code, message }).await;
    let _ = sink.close().await;
}

async fn send(
    sink: &mut SplitSink<WebSocket, Message>,
    message: &ServerMessage,
) -> Result<(), axum::Error> {
    let json = serde_json::to_string(message).expect("server messages always serialize");
    sink.send(Message::Text(json.into())).await
}
