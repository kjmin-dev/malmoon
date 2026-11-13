//! The connection to the gateway.
//!
//! GPUI runs its own executors and the WebSocket client needs tokio, so the connection lives on a
//! dedicated thread with its own tokio runtime and reports to the UI through a channel.

use std::time::Duration;

use anyhow::{Context as _, bail};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use futures_util::{SinkExt, StreamExt};
use malmoon_core::protocol::{
    CaptionUpdate, ClientKind, ClientMessage, PROTOCOL_VERSION, SESSION_PATH, ServerMessage,
};
use tokio_tungstenite::tungstenite::Message;

const RETRY_MIN: Duration = Duration::from_secs(1);
const RETRY_MAX: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub enum SessionEvent {
    Connecting,
    Ready,
    Caption(CaptionUpdate),
    Disconnected { reason: String },
}

/// Connects to the gateway at `base_url` (for example `ws://127.0.0.1:8787`) and keeps
/// reconnecting until the returned receiver is dropped.
pub fn spawn(base_url: String) -> UnboundedReceiver<SessionEvent> {
    let (events, receiver) = unbounded();
    std::thread::Builder::new()
        .name("malmoon-session".into())
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("the session runtime could not start")
                .block_on(run(base_url, events));
        })
        .expect("the session thread could not start");
    receiver
}

async fn run(base_url: String, events: UnboundedSender<SessionEvent>) {
    let url = format!("{}{SESSION_PATH}", base_url.trim_end_matches('/'));
    let mut retry = RETRY_MIN;
    loop {
        if events.unbounded_send(SessionEvent::Connecting).is_err() {
            return;
        }
        let mut was_ready = false;
        let reason = match session(&url, &events, &mut was_ready).await {
            Ok(()) => "the gateway closed the session".to_owned(),
            Err(error) => format!("{error:#}"),
        };
        if events
            .unbounded_send(SessionEvent::Disconnected { reason })
            .is_err()
        {
            return;
        }
        if was_ready {
            retry = RETRY_MIN;
        }
        tokio::time::sleep(retry).await;
        retry = (retry * 2).min(RETRY_MAX);
    }
}

async fn session(
    url: &str,
    events: &UnboundedSender<SessionEvent>,
    was_ready: &mut bool,
) -> anyhow::Result<()> {
    let (mut socket, _) = tokio_tungstenite::connect_async(url)
        .await
        .with_context(|| format!("could not reach the gateway at {url}"))?;

    let hello = ClientMessage::Hello {
        protocol_version: PROTOCOL_VERSION,
        client: ClientKind::Desktop,
    };
    socket
        .send(Message::Text(serde_json::to_string(&hello)?.into()))
        .await?;

    while let Some(frame) = socket.next().await {
        let event = match frame? {
            Message::Text(text) => match serde_json::from_str(&text)? {
                ServerMessage::Ready { .. } => {
                    *was_ready = true;
                    SessionEvent::Ready
                }
                ServerMessage::Caption { caption } => SessionEvent::Caption(caption),
                ServerMessage::Error { code, message } => {
                    bail!("the gateway refused the session ({code:?}): {message}")
                }
            },
            Message::Close(_) => break,
            _ => continue,
        };
        if events.unbounded_send(event).is_err() {
            break;
        }
    }
    Ok(())
}
