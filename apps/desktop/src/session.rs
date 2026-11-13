//! The connection to the gateway, and the account it runs under.
//!
//! GPUI runs its own executors and the WebSocket client needs tokio, so the connection lives on a
//! dedicated thread with its own tokio runtime. It reports to the UI through one channel and
//! takes commands from it through another.

use std::time::Duration;

use anyhow::Context as _;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use futures_util::{SinkExt, StreamExt};
use malmoon_core::protocol::{
    CaptionUpdate, ClientKind, ClientMessage, PROTOCOL_VERSION, SESSION_PATH, ServerMessage,
};
use tokio::sync::mpsc as commands;
use tokio_tungstenite::tungstenite::{self, Message, client::IntoClientRequest, http::StatusCode};

use crate::auth::{self, TokenStore};

const RETRY_MIN: Duration = Duration::from_secs(1);
const RETRY_MAX: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub enum SessionEvent {
    /// Nobody is signed in. `problem` says why the last attempt or session ended, if it did.
    SignedOut {
        problem: Option<String>,
    },
    /// The browser is open and the app is waiting for the person to finish there.
    SigningIn,
    Connecting,
    Ready,
    Caption(CaptionUpdate),
    Disconnected {
        reason: String,
    },
}

#[derive(Debug)]
pub enum Command {
    SignIn,
    SignOut,
}

pub struct Config {
    /// The gateway's WebSocket base URL, for example `ws://127.0.0.1:8787`.
    pub gateway_url: String,
    /// The web app, where signing in happens.
    pub web_url: String,
}

pub struct Handle {
    pub events: UnboundedReceiver<SessionEvent>,
    pub commands: commands::UnboundedSender<Command>,
}

/// Starts the session thread. It runs until the returned handle is dropped.
pub fn spawn(config: Config) -> Handle {
    let (events, receiver) = unbounded();
    let (sender, commands) = commands::unbounded_channel();
    std::thread::Builder::new()
        .name("malmoon-session".into())
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("the session runtime could not start")
                .block_on(run(config, events, commands));
        })
        .expect("the session thread could not start");
    Handle {
        events: receiver,
        commands: sender,
    }
}

/// Why a connection ended.
enum Ended {
    /// The gateway does not know the token: it expired or was revoked.
    Unauthorized,
    Lost(String),
    SignOut,
    /// The UI is gone.
    Shutdown,
}

async fn run(
    config: Config,
    events: UnboundedSender<SessionEvent>,
    mut commands: commands::UnboundedReceiver<Command>,
) {
    let url = format!("{}{SESSION_PATH}", config.gateway_url.trim_end_matches('/'));
    // ws://host -> http://host, wss://host -> https://host
    let gateway = config
        .gateway_url
        .trim_end_matches('/')
        .replacen("ws", "http", 1);
    let store = TokenStore::for_gateway(&gateway);
    let mut problem = None;
    let mut retry = RETRY_MIN;

    loop {
        let Some(token) = store.load() else {
            if events
                .unbounded_send(SessionEvent::SignedOut {
                    problem: problem.take(),
                })
                .is_err()
            {
                return;
            }
            loop {
                match commands.recv().await {
                    Some(Command::SignIn) => break,
                    Some(Command::SignOut) => {}
                    None => return,
                }
            }
            let _ = events.unbounded_send(SessionEvent::SigningIn);
            let signed_in = match auth::sign_in(&gateway, &config.web_url).await {
                Ok(token) => store.save(&token),
                Err(error) => Err(error),
            };
            if let Err(error) = signed_in {
                problem = Some(format!("{error:#}"));
            }
            continue;
        };

        if events.unbounded_send(SessionEvent::Connecting).is_err() {
            return;
        }
        let mut was_ready = false;
        let ended = tokio::select! {
            ended = session(&url, &token, &events, &mut was_ready) => ended,
            ended = sign_out_requested(&mut commands) => ended,
        };
        if was_ready {
            retry = RETRY_MIN;
        }
        match ended {
            Ended::Shutdown => return,
            Ended::SignOut => {
                auth::sign_out(&gateway, &token).await;
                store.clear();
            }
            Ended::Unauthorized => {
                store.clear();
                problem = Some("로그인이 만료되었습니다. 다시 로그인해 주세요.".to_owned());
            }
            Ended::Lost(reason) => {
                if events
                    .unbounded_send(SessionEvent::Disconnected { reason })
                    .is_err()
                {
                    return;
                }
                tokio::select! {
                    () = tokio::time::sleep(retry) => {}
                    ended = sign_out_requested(&mut commands) => match ended {
                        Ended::Shutdown => return,
                        _ => {
                            auth::sign_out(&gateway, &token).await;
                            store.clear();
                        }
                    },
                }
                retry = (retry * 2).min(RETRY_MAX);
            }
        }
    }
}

async fn sign_out_requested(commands: &mut commands::UnboundedReceiver<Command>) -> Ended {
    loop {
        match commands.recv().await {
            Some(Command::SignOut) => return Ended::SignOut,
            Some(Command::SignIn) => {}
            None => return Ended::Shutdown,
        }
    }
}

async fn session(
    url: &str,
    token: &str,
    events: &UnboundedSender<SessionEvent>,
    was_ready: &mut bool,
) -> Ended {
    match stream(url, token, events, was_ready).await {
        Ok(ended) => ended,
        Err(error) => match error.downcast_ref::<tungstenite::Error>() {
            Some(tungstenite::Error::Http(response))
                if response.status() == StatusCode::UNAUTHORIZED =>
            {
                Ended::Unauthorized
            }
            _ => Ended::Lost(format!("{error:#}")),
        },
    }
}

async fn stream(
    url: &str,
    token: &str,
    events: &UnboundedSender<SessionEvent>,
    was_ready: &mut bool,
) -> anyhow::Result<Ended> {
    let mut request = url
        .into_client_request()
        .with_context(|| format!("{url:?} is not a WebSocket URL"))?;
    request
        .headers_mut()
        .insert("authorization", format!("Bearer {token}").parse()?);
    let (mut socket, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|error| match error {
            // Keep the type: the caller tells a refused token from an unreachable gateway by it.
            refused @ tungstenite::Error::Http(_) => anyhow::Error::new(refused),
            other => {
                anyhow::Error::new(other).context(format!("could not reach the gateway at {url}"))
            }
        })?;

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
                    return Ok(Ended::Lost(format!(
                        "the gateway refused the session ({code:?}): {message}"
                    )));
                }
            },
            Message::Close(_) => break,
            _ => continue,
        };
        if events.unbounded_send(event).is_err() {
            return Ok(Ended::Shutdown);
        }
    }
    Ok(Ended::Lost("the gateway closed the session".to_owned()))
}
