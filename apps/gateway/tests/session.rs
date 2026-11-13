//! End-to-end tests against a gateway listening on a real socket.

use futures_util::{SinkExt, StreamExt};
use malmoon_auth::{Auth, AuthConfig, EmailVerified, MailFuture, Mailer};
use malmoon_core::protocol::{
    CaptionStage, ClientKind, ClientMessage, ErrorCode, PROTOCOL_VERSION, SESSION_PATH,
    ServerMessage,
};
use malmoon_gateway::{Config, router};
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{self, Message, client::IntoClientRequest},
};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Keeps the last sign-in code instead of sending it.
#[derive(Clone, Default)]
struct Outbox(Arc<Mutex<String>>);

impl Mailer for Outbox {
    fn send_login_code<'a>(&'a self, _to: &'a str, code: &'a str) -> MailFuture<'a> {
        *self.0.lock().unwrap() = code.to_owned();
        Box::pin(async { Ok(()) })
    }
}

/// Starts a gateway and returns its session URL with the token of a signed-in desktop user.
async fn start(pool: PgPool, config: Config) -> (String, String) {
    let outbox = Outbox::default();
    let auth = Auth::new(pool, AuthConfig::local(), outbox.clone());
    let challenge = auth.start_email("mina@example.com", None).await.unwrap();
    let code = outbox.0.lock().unwrap().clone();
    let verified = auth
        .verify_email(challenge.challenge_id, &code, ClientKind::Desktop, None)
        .await
        .unwrap();
    let EmailVerified::SignedIn(session) = verified else {
        panic!("expected a new session");
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router(config, auth)).await.unwrap() });
    (format!("ws://{addr}{SESSION_PATH}"), session.token)
}

async fn open(url: &str, token: Option<&str>) -> Result<Socket, tungstenite::Error> {
    let mut request = url.into_client_request().unwrap();
    if let Some(token) = token {
        let value = format!("Bearer {token}").parse().unwrap();
        request.headers_mut().insert("authorization", value);
    }
    tokio_tungstenite::connect_async(request)
        .await
        .map(|(socket, _)| socket)
}

async fn connect(pool: PgPool, config: Config) -> Socket {
    let (url, token) = start(pool, config).await;
    open(&url, Some(&token)).await.unwrap()
}

async fn send(socket: &mut Socket, message: &ClientMessage) {
    let json = serde_json::to_string(message).unwrap();
    socket.send(Message::Text(json.into())).await.unwrap();
}

async fn receive(socket: &mut Socket) -> ServerMessage {
    loop {
        match socket
            .next()
            .await
            .expect("the gateway closed the socket")
            .unwrap()
        {
            Message::Text(text) => return serde_json::from_str(&text).unwrap(),
            Message::Ping(_) | Message::Pong(_) => {}
            other => panic!("unexpected frame: {other:?}"),
        }
    }
}

fn hello(protocol_version: u16) -> ClientMessage {
    ClientMessage::Hello {
        protocol_version,
        client: ClientKind::Desktop,
    }
}

#[sqlx::test(migrator = "malmoon_auth::MIGRATOR")]
async fn hello_is_answered_with_ready(pool: PgPool) {
    let mut socket = connect(pool, Config::default()).await;
    send(&mut socket, &hello(PROTOCOL_VERSION)).await;
    match receive(&mut socket).await {
        ServerMessage::Ready {
            protocol_version,
            session_id,
        } => {
            assert_eq!(protocol_version, PROTOCOL_VERSION);
            assert!(!session_id.is_empty());
        }
        other => panic!("expected ready, got {other:?}"),
    }
}

#[sqlx::test(migrator = "malmoon_auth::MIGRATOR")]
async fn an_unsupported_protocol_version_is_rejected(pool: PgPool) {
    let mut socket = connect(pool, Config::default()).await;
    send(&mut socket, &hello(PROTOCOL_VERSION + 1)).await;
    match receive(&mut socket).await {
        ServerMessage::Error { code, .. } => assert_eq!(code, ErrorCode::UnsupportedProtocol),
        other => panic!("expected an error, got {other:?}"),
    }
}

#[sqlx::test(migrator = "malmoon_auth::MIGRATOR")]
async fn anything_but_hello_first_is_rejected(pool: PgPool) {
    let mut socket = connect(pool, Config::default()).await;
    socket.send(Message::Text("{}".into())).await.unwrap();
    match receive(&mut socket).await {
        ServerMessage::Error { code, .. } => assert_eq!(code, ErrorCode::ExpectedHello),
        other => panic!("expected an error, got {other:?}"),
    }
}

#[sqlx::test(migrator = "malmoon_auth::MIGRATOR")]
async fn the_demo_feed_sends_a_draft_first(pool: PgPool) {
    let mut socket = connect(
        pool,
        Config {
            demo_captions: true,
        },
    )
    .await;
    send(&mut socket, &hello(PROTOCOL_VERSION)).await;
    assert!(matches!(
        receive(&mut socket).await,
        ServerMessage::Ready { .. }
    ));
    match receive(&mut socket).await {
        ServerMessage::Caption { caption } => {
            assert_eq!(caption.segment_id, 1);
            assert_eq!(caption.stage, CaptionStage::Draft);
            assert_eq!(caption.original, "Thanks");
        }
        other => panic!("expected a caption, got {other:?}"),
    }
}

#[sqlx::test(migrator = "malmoon_auth::MIGRATOR")]
async fn a_session_needs_a_signed_in_user(pool: PgPool) {
    let (url, token) = start(pool, Config::default()).await;
    for refused in [None, Some("mms_not-a-session")] {
        match open(&url, refused).await {
            Err(tungstenite::Error::Http(response)) => assert_eq!(response.status(), 401),
            other => panic!("expected a 401 for {refused:?}, got {other:?}"),
        }
    }
    let mut socket = open(&url, Some(&token)).await.unwrap();
    send(&mut socket, &hello(PROTOCOL_VERSION)).await;
    assert!(matches!(
        receive(&mut socket).await,
        ServerMessage::Ready { .. }
    ));
}
