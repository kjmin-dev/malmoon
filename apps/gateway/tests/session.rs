//! End-to-end tests against a gateway listening on a real socket.

use futures_util::{SinkExt, StreamExt};
use malmoon_core::protocol::{
    CaptionStage, ClientKind, ClientMessage, ErrorCode, PROTOCOL_VERSION, SESSION_PATH,
    ServerMessage,
};
use malmoon_gateway::{Config, router};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

async fn connect(config: Config) -> Socket {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router(config)).await.unwrap() });
    let (socket, _) = tokio_tungstenite::connect_async(format!("ws://{addr}{SESSION_PATH}"))
        .await
        .unwrap();
    socket
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

#[tokio::test]
async fn hello_is_answered_with_ready() {
    let mut socket = connect(Config::default()).await;
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

#[tokio::test]
async fn an_unsupported_protocol_version_is_rejected() {
    let mut socket = connect(Config::default()).await;
    send(&mut socket, &hello(PROTOCOL_VERSION + 1)).await;
    match receive(&mut socket).await {
        ServerMessage::Error { code, .. } => assert_eq!(code, ErrorCode::UnsupportedProtocol),
        other => panic!("expected an error, got {other:?}"),
    }
}

#[tokio::test]
async fn anything_but_hello_first_is_rejected() {
    let mut socket = connect(Config::default()).await;
    socket.send(Message::Text("{}".into())).await.unwrap();
    match receive(&mut socket).await {
        ServerMessage::Error { code, .. } => assert_eq!(code, ErrorCode::ExpectedHello),
        other => panic!("expected an error, got {other:?}"),
    }
}

#[tokio::test]
async fn the_demo_feed_sends_a_draft_first() {
    let mut socket = connect(Config {
        demo_captions: true,
    })
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
