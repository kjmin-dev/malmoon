//! The HTTP surface of the account system.

use axum::{
    Form, Json, Router,
    extract::{FromRef, FromRequestParts, Path, Query, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header, request::Parts},
    response::{IntoResponse, Redirect, Response},
    routing::{delete, get, post},
};
use malmoon_core::protocol::ClientKind;
use serde::{Deserialize, Serialize};
use serde_json::json;
use time::OffsetDateTime;
use tower_http::cors::{AllowOrigin, CorsLayer};
use uuid::Uuid;

use crate::{
    Auth, EMAIL_CODE_SECONDS, EMAIL_RESEND_SECONDS, EmailVerified, OAuthStart, SignedIn,
    error::AuthError,
    model::{Account, Provider, SessionInfo},
};

/// Web sessions live in this cookie. Scripts cannot read it, which is the point: a script
/// injected into the page cannot carry the session away.
pub const SESSION_COOKIE: &str = "mm_session";

impl Auth {
    /// The account routes, under `/v1/auth` and `/v1/me`.
    pub fn router(&self) -> Router {
        let origins = self
            .config()
            .web_origins
            .iter()
            .filter_map(|origin| HeaderValue::from_str(origin).ok());
        let cors = CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_credentials(true)
            .allow_methods([Method::GET, Method::POST, Method::DELETE])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);
        Router::new()
            .route("/v1/auth/providers", get(providers))
            .route("/v1/auth/oauth/{provider}/start", post(start_oauth))
            .route(
                "/v1/auth/oauth/{provider}/callback",
                get(callback_query).post(callback_form),
            )
            .route("/v1/auth/token", post(exchange))
            .route("/v1/auth/handoff", post(handoff))
            .route("/v1/auth/email/start", post(start_email))
            .route("/v1/auth/email/verify", post(verify_email))
            .route("/v1/auth/logout", post(logout))
            .route("/v1/me", get(me))
            .route("/v1/me/identities/{id}", delete(unlink))
            .layer(cors)
            .with_state(self.clone())
    }
}

// ---------------------------------------------------------------- who is asking

/// The signed-in session of a request: a bearer token from the desktop and mobile apps, or the
/// session cookie from the web app. Rejects the request with 401 when there is none.
#[derive(Clone, Copy, Debug)]
pub struct CurrentSession {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub client: ClientKind,
}

/// Like [`CurrentSession`], for routes that also serve people who are not signed in.
struct MaybeSession(Option<CurrentSession>);

impl<S> FromRequestParts<S> for CurrentSession
where
    Auth: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        session_of(parts, &Auth::from_ref(state))
            .await?
            .ok_or(AuthError::Unauthorized)
    }
}

impl<S> FromRequestParts<S> for MaybeSession
where
    Auth: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Ok(MaybeSession(
            session_of(parts, &Auth::from_ref(state)).await?,
        ))
    }
}

async fn session_of(parts: &Parts, auth: &Auth) -> Result<Option<CurrentSession>, AuthError> {
    let found = if let Some(token) = bearer_token(&parts.headers) {
        auth.authenticate(token).await?
    } else if let Some(token) = cookie(&parts.headers, SESSION_COOKIE) {
        // A browser attaches the cookie to requests that other sites trigger. `SameSite=Lax`
        // stops most of those; checking the origin stops the rest, including WebSocket
        // handshakes, which are GET requests that other sites may open.
        let reads_only = matches!(parts.method, Method::GET | Method::HEAD)
            && !parts.headers.contains_key(header::UPGRADE);
        if !reads_only {
            require_web_origin(&parts.headers, auth)?;
        }
        auth.authenticate(token).await?
    } else {
        None
    };
    Ok(found.map(
        |SessionInfo {
             session_id,
             user_id,
             client,
         }| CurrentSession {
            session_id,
            user_id,
            client,
        },
    ))
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    scheme
        .eq_ignore_ascii_case("bearer")
        .then_some(token.trim())
}

fn cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find_map(|(key, value)| (key == name).then_some(value))
}

fn require_web_origin(headers: &HeaderMap, auth: &Auth) -> Result<(), AuthError> {
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok());
    if origin.is_some_and(|origin| auth.config().is_web_origin(origin)) {
        Ok(())
    } else {
        Err(AuthError::ForbiddenOrigin)
    }
}

fn session_cookie(auth: &Auth, token: Option<&str>) -> HeaderValue {
    // The server decides when a session ends, so the cookie only has to outlive it.
    let max_age = if token.is_some() { 400 * 24 * 3600 } else { 0 };
    let secure = if auth.config().secure_cookies() {
        "; Secure"
    } else {
        ""
    };
    let value = format!(
        "{SESSION_COOKIE}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure}",
        token.unwrap_or_default()
    );
    HeaderValue::from_str(&value).expect("session tokens are URL-safe ASCII")
}

fn label(headers: &HeaderMap) -> Option<String> {
    let agent = headers.get(header::USER_AGENT)?.to_str().ok()?;
    Some(agent.chars().take(200).collect())
}

// ---------------------------------------------------------------- responses

#[derive(Serialize)]
struct SessionBody {
    #[serde(flatten)]
    account: Account,
    /// Null when the request added a sign-in method to the current session's account.
    session: Option<SessionOut>,
}

#[derive(Serialize)]
struct SessionOut {
    /// Null for the web app, whose session is in the cookie instead.
    token: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    expires_at: OffsetDateTime,
}

/// Hands a new session to its client: the web app gets a cookie that scripts cannot read, the
/// desktop and mobile apps get the token to keep in the operating system's keychain.
fn signed_in(auth: &Auth, headers: &HeaderMap, signed_in: SignedIn) -> Result<Response, AuthError> {
    let SignedIn {
        account,
        client,
        token,
        expires_at,
    } = signed_in;
    if client == ClientKind::Web {
        require_web_origin(headers, auth)?;
        let body = SessionBody {
            account,
            session: Some(SessionOut {
                token: None,
                expires_at,
            }),
        };
        Ok((
            [(header::SET_COOKIE, session_cookie(auth, Some(&token)))],
            Json(body),
        )
            .into_response())
    } else {
        let body = SessionBody {
            account,
            session: Some(SessionOut {
                token: Some(token),
                expires_at,
            }),
        };
        Ok(Json(body).into_response())
    }
}

// ---------------------------------------------------------------- handlers

async fn providers(State(auth): State<Auth>) -> Json<serde_json::Value> {
    Json(json!({ "providers": auth.providers() }))
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Intent {
    #[default]
    SignIn,
    /// Add the sign-in method to the account of the current session.
    Link,
}

fn link_target(intent: Intent, session: Option<CurrentSession>) -> Result<Option<Uuid>, AuthError> {
    match intent {
        Intent::SignIn => Ok(None),
        Intent::Link => Ok(Some(session.ok_or(AuthError::Unauthorized)?.user_id)),
    }
}

fn social(provider: &str) -> Result<Provider, AuthError> {
    match provider.parse() {
        Ok(Provider::Email) | Err(_) => Err(AuthError::InvalidRequest(format!(
            "{provider:?} is not a social provider"
        ))),
        Ok(provider) => Ok(provider),
    }
}

#[derive(Deserialize)]
struct StartOAuthBody {
    client: ClientKind,
    redirect_uri: String,
    code_challenge: String,
    state: Option<String>,
    #[serde(default)]
    intent: Intent,
}

async fn start_oauth(
    State(auth): State<Auth>,
    Path(provider): Path<String>,
    MaybeSession(session): MaybeSession,
    Json(body): Json<StartOAuthBody>,
) -> Result<Json<serde_json::Value>, AuthError> {
    let authorization_url = auth
        .start_oauth(OAuthStart {
            provider: social(&provider)?,
            client: body.client,
            redirect_uri: &body.redirect_uri,
            code_challenge: &body.code_challenge,
            state: body.state.as_deref(),
            link_user: link_target(body.intent, session)?,
        })
        .await?;
    Ok(Json(json!({ "authorization_url": authorization_url })))
}

#[derive(Debug, Deserialize)]
pub(crate) struct Callback {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    /// Apple's only chance to tell us the person's name: a JSON form field on the first
    /// authorization. It is not signed, so it is used for display and nothing else.
    pub user: Option<String>,
}

pub(crate) fn apple_display_name(user: &str) -> Option<String> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Name {
        first_name: Option<String>,
        last_name: Option<String>,
    }
    #[derive(Deserialize)]
    struct User {
        name: Option<Name>,
    }
    let name = serde_json::from_str::<User>(user).ok()?.name?;
    let full = [name.first_name, name.last_name]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
    let full: String = full.trim().chars().take(100).collect();
    (!full.is_empty()).then_some(full)
}

async fn callback_query(
    State(auth): State<Auth>,
    Path(provider): Path<String>,
    Query(callback): Query<Callback>,
) -> Response {
    callback_response(auth, provider, callback).await
}

async fn callback_form(
    State(auth): State<Auth>,
    Path(provider): Path<String>,
    Form(callback): Form<Callback>,
) -> Response {
    callback_response(auth, provider, callback).await
}

async fn callback_response(auth: Auth, provider: String, callback: Callback) -> Response {
    let redirect = match social(&provider) {
        Ok(provider) => auth.finish_oauth(provider, callback).await,
        Err(error) => Err(error),
    };
    match redirect {
        // 303 turns Apple's POST into a GET of the app's redirect URI.
        Ok(url) => Redirect::to(&url).into_response(),
        // There is no app to return to, so say it to the person in the browser.
        Err(error) => {
            if let AuthError::Internal(cause) = &error {
                tracing::error!(error = format!("{cause:#}"), "provider callback failed");
            }
            let page = "로그인 요청이 만료되었거나 이미 사용되었습니다. 앱으로 돌아가 다시 시도해 주세요.\n\
                        This sign-in attempt has expired or was already used. Return to the app and try again.";
            (
                StatusCode::BAD_REQUEST,
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                page,
            )
                .into_response()
        }
    }
}

#[derive(Deserialize)]
struct ExchangeBody {
    code: String,
    code_verifier: String,
}

async fn exchange(
    State(auth): State<Auth>,
    headers: HeaderMap,
    Json(body): Json<ExchangeBody>,
) -> Result<Response, AuthError> {
    let session = auth
        .exchange_login_code(&body.code, &body.code_verifier, label(&headers).as_deref())
        .await?;
    signed_in(&auth, &headers, session)
}

#[derive(Deserialize)]
struct HandoffBody {
    redirect_uri: String,
    code_challenge: String,
    state: Option<String>,
}

async fn handoff(
    State(auth): State<Auth>,
    session: CurrentSession,
    Json(body): Json<HandoffBody>,
) -> Result<Json<serde_json::Value>, AuthError> {
    let redirect_url = auth
        .handoff(
            session.user_id,
            &body.redirect_uri,
            &body.code_challenge,
            body.state.as_deref(),
        )
        .await?;
    Ok(Json(json!({ "redirect_url": redirect_url })))
}

#[derive(Deserialize)]
struct StartEmailBody {
    email: String,
    #[serde(default)]
    intent: Intent,
}

async fn start_email(
    State(auth): State<Auth>,
    MaybeSession(session): MaybeSession,
    Json(body): Json<StartEmailBody>,
) -> Result<Json<serde_json::Value>, AuthError> {
    let challenge = auth
        .start_email(&body.email, link_target(body.intent, session)?)
        .await?;
    Ok(Json(json!({
        "challenge_id": challenge.challenge_id,
        "expires_in": EMAIL_CODE_SECONDS,
        "resend_after": EMAIL_RESEND_SECONDS,
    })))
}

#[derive(Deserialize)]
struct VerifyEmailBody {
    challenge_id: Uuid,
    code: String,
    client: ClientKind,
}

async fn verify_email(
    State(auth): State<Auth>,
    headers: HeaderMap,
    Json(body): Json<VerifyEmailBody>,
) -> Result<Response, AuthError> {
    if body.client == ClientKind::Web {
        // Before the code is spent, not after.
        require_web_origin(&headers, &auth)?;
    }
    match auth
        .verify_email(
            body.challenge_id,
            &body.code,
            body.client,
            label(&headers).as_deref(),
        )
        .await?
    {
        EmailVerified::SignedIn(session) => signed_in(&auth, &headers, session),
        EmailVerified::Linked(account) => Ok(Json(SessionBody {
            account,
            session: None,
        })
        .into_response()),
    }
}

async fn logout(State(auth): State<Auth>, session: CurrentSession) -> Result<Response, AuthError> {
    auth.sign_out(session.session_id).await?;
    Ok((
        StatusCode::NO_CONTENT,
        [(header::SET_COOKIE, session_cookie(&auth, None))],
    )
        .into_response())
}

async fn me(State(auth): State<Auth>, session: CurrentSession) -> Result<Json<Account>, AuthError> {
    Ok(Json(auth.account(session.user_id).await?))
}

async fn unlink(
    State(auth): State<Auth>,
    session: CurrentSession,
    Path(identity): Path<Uuid>,
) -> Result<StatusCode, AuthError> {
    auth.unlink(session.user_id, identity).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_session_cookie_among_others() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("theme=dark; mm_session=mms_abc; other=1"),
        );
        assert_eq!(cookie(&headers, SESSION_COOKIE), Some("mms_abc"));
        assert_eq!(cookie(&headers, "missing"), None);
    }

    #[test]
    fn reads_bearer_tokens_whatever_the_case_of_the_scheme() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("bearer mms_abc"),
        );
        assert_eq!(bearer_token(&headers), Some("mms_abc"));
        headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Basic abc"));
        assert_eq!(bearer_token(&headers), None);
    }

    #[test]
    fn apple_names_are_joined_and_optional() {
        assert_eq!(
            apple_display_name(
                r#"{"name":{"firstName":"Mina","lastName":"Kim"},"email":"m@example.com"}"#
            )
            .as_deref(),
            Some("Mina Kim")
        );
        assert_eq!(
            apple_display_name(r#"{"name":{"firstName":"Mina"}}"#).as_deref(),
            Some("Mina")
        );
        assert_eq!(apple_display_name(r#"{"email":"m@example.com"}"#), None);
        assert_eq!(apple_display_name("not json"), None);
    }
}
