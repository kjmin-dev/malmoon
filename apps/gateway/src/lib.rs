//! The Malmoon real-time gateway.
//!
//! Clients sign in through the account routes of `malmoon-auth`, then open one WebSocket per
//! meeting session. The gateway will route their audio to the speech models and send captions
//! back. Only the session handshake exists so far: no model is connected yet, and captions come
//! from a scripted demo feed when one is enabled.

#![forbid(unsafe_code)]

mod demo;
mod session;

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use axum::{Router, extract::FromRef, routing::get};
use malmoon_auth::Auth;
use malmoon_core::protocol::SESSION_PATH;
use tower_http::trace::TraceLayer;

#[derive(Debug, Clone, Default)]
pub struct Config {
    /// Send scripted captions to every session. For developing clients before a speech model is
    /// connected; never enable it in production.
    pub demo_captions: bool,
}

#[derive(Clone)]
pub(crate) struct AppState {
    config: Config,
    auth: Auth,
    next_session: Arc<AtomicU64>,
}

impl AppState {
    fn next_session_id(&self) -> String {
        format!("s-{}", self.next_session.fetch_add(1, Ordering::Relaxed))
    }
}

impl FromRef<AppState> for Auth {
    fn from_ref(state: &AppState) -> Self {
        state.auth.clone()
    }
}

pub fn router(config: Config, auth: Auth) -> Router {
    let state = AppState {
        config,
        auth: auth.clone(),
        next_session: Arc::new(AtomicU64::new(1)),
    };
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route(SESSION_PATH, get(session::upgrade))
        .with_state(state)
        .merge(auth.router())
        .layer(TraceLayer::new_for_http())
}
