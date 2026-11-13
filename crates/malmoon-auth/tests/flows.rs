//! The account API end to end, against a real Postgres and a stand-in for the social providers.
//!
//! The stand-in plays the provider's token and userinfo endpoints. The browser's trip to the
//! provider is skipped: each test reads `state` and `nonce` from the authorization URL and calls
//! the callback itself, as the provider's redirect would.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{
    Form, Json, Router,
    extract::State,
    http::HeaderMap,
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use malmoon_auth::{
    Auth, AuthConfig, ClientSecret, MailFuture, Mailer, Provider, ProviderConfig, pkce_challenge,
};
use reqwest::{StatusCode, header};
use serde_json::{Value, json};
use sqlx::PgPool;
use url::Url;

const WEB_ORIGIN: &str = "http://localhost:5173";
const WEB_REDIRECT: &str = "http://localhost:5173/auth/callback";
const DESKTOP_REDIRECT: &str = "http://127.0.0.1:49152/callback";
const VERIFIER: &str = "a-verifier-that-only-this-app-instance-knows";

#[derive(Clone, Default)]
struct Outbox(Arc<Mutex<Vec<(String, String)>>>);

impl Mailer for Outbox {
    fn send_login_code<'a>(&'a self, to: &'a str, code: &'a str) -> MailFuture<'a> {
        self.0
            .lock()
            .unwrap()
            .push((to.to_owned(), code.to_owned()));
        Box::pin(async { Ok(()) })
    }
}

/// What the stand-in provider will answer for each authorization code and access token.
#[derive(Clone, Default)]
struct Idp {
    id_tokens: Arc<Mutex<HashMap<String, Value>>>,
    userinfo: Arc<Mutex<HashMap<String, Value>>>,
    token_requests: Arc<Mutex<Vec<HashMap<String, String>>>>,
}

async fn idp_token(
    State(idp): State<Idp>,
    Form(form): Form<HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    idp.token_requests.lock().unwrap().push(form.clone());
    let claims = idp
        .id_tokens
        .lock()
        .unwrap()
        .remove(&form["code"])
        .ok_or(StatusCode::BAD_REQUEST)?;
    let segment = |value: &Value| URL_SAFE_NO_PAD.encode(value.to_string());
    let id_token = format!(
        "{}.{}.signature",
        segment(&json!({ "alg": "RS256" })),
        segment(&claims)
    );
    Ok(Json(
        json!({ "id_token": id_token, "access_token": format!("access-{}", form["code"]), "token_type": "Bearer" }),
    ))
}

async fn idp_userinfo(
    State(idp): State<Idp>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    let token = headers[header::AUTHORIZATION]
        .to_str()
        .unwrap()
        .trim_start_matches("Bearer ")
        .to_owned();
    idp.userinfo
        .lock()
        .unwrap()
        .get(&token)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::UNAUTHORIZED)
}

struct App {
    base: String,
    idp_base: String,
    http: reqwest::Client,
    outbox: Outbox,
    idp: Idp,
    codes: std::sync::atomic::AtomicU32,
}

struct Person<'a> {
    sub: &'a str,
    email: Option<&'a str>,
    /// The `email_verified` claim of the ID token; `None` leaves it out.
    verified: Option<bool>,
}

async fn serve(router: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    base
}

impl App {
    async fn start(pool: PgPool) -> Self {
        let idp = Idp::default();
        let idp_base = serve(
            Router::new()
                .route("/token", post(idp_token))
                .route("/userinfo", get(idp_userinfo))
                .with_state(idp.clone()),
        )
        .await;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let provider = |name: &str, userinfo: bool| ProviderConfig {
            client_id: format!("client-{name}"),
            secret: ClientSecret::Static("provider-secret".to_owned()),
            authorize_url: format!("{idp_base}/authorize"),
            token_url: format!("{idp_base}/token"),
            userinfo_url: userinfo.then(|| format!("{idp_base}/userinfo")),
            issuers: vec![idp_base.clone()],
            scope: "openid email".to_owned(),
            pkce: true,
            form_post: false,
            email_always_verified: false,
        };
        let mut config = AuthConfig::local();
        config.public_url = base.clone();
        config
            .providers
            .insert(Provider::Google, provider("google", false));
        config
            .providers
            .insert(Provider::Kakao, provider("kakao", true));
        config
            .providers
            .insert(Provider::Line, provider("line", false));

        let outbox = Outbox::default();
        let auth = Auth::new(pool, config, outbox.clone());
        tokio::spawn(async move { axum::serve(listener, auth.router()).await.unwrap() });

        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        Self {
            base,
            idp_base,
            http,
            outbox,
            idp,
            codes: Default::default(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    async fn post(&self, path: &str, bearer: Option<&str>, body: Value) -> (StatusCode, Value) {
        let mut request = self.http.post(self.url(path)).json(&body);
        if let Some(token) = bearer {
            request = request.bearer_auth(token);
        }
        let response = request.send().await.unwrap();
        let status = response.status();
        (status, response.json().await.unwrap_or(Value::Null))
    }

    async fn get(&self, path: &str, bearer: &str) -> (StatusCode, Value) {
        let response = self
            .http
            .get(self.url(path))
            .bearer_auth(bearer)
            .send()
            .await
            .unwrap();
        let status = response.status();
        (status, response.json().await.unwrap_or(Value::Null))
    }

    fn last_code(&self, email: &str) -> String {
        let outbox = self.outbox.0.lock().unwrap();
        outbox
            .iter()
            .rev()
            .find(|(to, _)| to == email)
            .expect("a code was sent")
            .1
            .clone()
    }

    /// Requests a code for the address and returns the challenge id.
    async fn email_start(&self, email: &str, bearer: Option<&str>) -> String {
        let intent = if bearer.is_some() { "link" } else { "sign_in" };
        let (status, body) = self
            .post(
                "/v1/auth/email/start",
                bearer,
                json!({ "email": email, "intent": intent }),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body["challenge_id"].as_str().unwrap().to_owned()
    }

    /// Signs in with an email code as the desktop app and returns the response body.
    async fn email_sign_in(&self, email: &str) -> Value {
        let challenge = self.email_start(email, None).await;
        let body = json!({ "challenge_id": challenge, "code": self.last_code(&email.to_lowercase()), "client": "desktop" });
        let (status, body) = self.post("/v1/auth/email/verify", None, body).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body
    }

    /// Runs a provider sign-in up to the redirect back to the app, and returns that redirect.
    async fn provider_redirect(
        &self,
        provider: &str,
        person: &Person<'_>,
        link_as: Option<&str>,
    ) -> Url {
        let body = json!({
            "client": "desktop",
            "redirect_uri": DESKTOP_REDIRECT,
            "code_challenge": pkce_challenge(VERIFIER),
            "state": "app-state",
            "intent": if link_as.is_some() { "link" } else { "sign_in" },
        });
        let (status, started) = self
            .post(&format!("/v1/auth/oauth/{provider}/start"), link_as, body)
            .await;
        assert_eq!(status, StatusCode::OK, "{started}");
        let authorize = Url::parse(started["authorization_url"].as_str().unwrap()).unwrap();
        assert!(
            authorize
                .as_str()
                .starts_with(&format!("{}/authorize", self.idp_base))
        );
        let param = |name: &str| {
            authorize
                .query_pairs()
                .find(|(key, _)| key == name)
                .unwrap()
                .1
                .into_owned()
        };
        assert_eq!(
            param("redirect_uri"),
            self.url(&format!("/v1/auth/oauth/{provider}/callback"))
        );

        let code = format!(
            "code-{}",
            self.codes
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        let mut claims = json!({
            "iss": self.idp_base,
            "aud": format!("client-{provider}"),
            "exp": 4_000_000_000u64,
            "sub": person.sub,
            "nonce": param("nonce"),
            "email": person.email,
        });
        if let Some(verified) = person.verified {
            claims["email_verified"] = json!(verified);
        }
        self.idp
            .id_tokens
            .lock()
            .unwrap()
            .insert(code.clone(), claims);
        self.callback(provider, &code, &param("state"))
            .await
            .expect("a redirect back to the app")
    }

    /// Plays the provider's redirect to our callback. `None` when the answer is not a redirect.
    async fn callback(&self, provider: &str, code: &str, state: &str) -> Option<Url> {
        let url = self.url(&format!("/v1/auth/oauth/{provider}/callback"));
        let response = self
            .http
            .get(url)
            .query(&[("code", code), ("state", state)])
            .send()
            .await
            .unwrap();
        if response.status() != StatusCode::SEE_OTHER {
            return None;
        }
        Some(Url::parse(response.headers()[header::LOCATION].to_str().unwrap()).unwrap())
    }

    /// A whole provider sign-in as the desktop app. Returns the token response body.
    async fn provider_sign_in(&self, provider: &str, person: &Person<'_>) -> Value {
        let redirect = self.provider_redirect(provider, person, None).await;
        assert!(
            redirect.as_str().starts_with(DESKTOP_REDIRECT),
            "{redirect}"
        );
        assert_eq!(query(&redirect, "state").as_deref(), Some("app-state"));
        let code =
            query(&redirect, "code").unwrap_or_else(|| panic!("no login code in {redirect}"));
        let (status, body) = self
            .post(
                "/v1/auth/token",
                None,
                json!({ "code": code, "code_verifier": VERIFIER }),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body
    }
}

fn query(url: &Url, name: &str) -> Option<String> {
    url.query_pairs()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
}

fn token(body: &Value) -> &str {
    body["session"]["token"].as_str().expect("a session token")
}

fn providers(body: &Value) -> Vec<&str> {
    body["identities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|identity| identity["provider"].as_str().unwrap())
        .collect()
}

const MINA: Person = Person {
    sub: "google-mina",
    email: Some("mina@example.com"),
    verified: Some(true),
};

// ---------------------------------------------------------------- email codes

#[sqlx::test(migrator = "malmoon_auth::MIGRATOR")]
async fn email_code_signs_up_then_signs_in_the_same_account(pool: PgPool) {
    let app = App::start(pool.clone()).await;
    let first = app.email_sign_in("Mina@Example.com").await;
    assert_eq!(providers(&first), ["email"]);
    assert_eq!(first["identities"][0]["email"], "mina@example.com");
    assert_eq!(first["identities"][0]["email_verified"], true);

    // The resend cooldown is per address; a different spelling of it is the same address.
    let (status, body) = app
        .post(
            "/v1/auth/email/start",
            None,
            json!({ "email": "mina@example.com" }),
        )
        .await;
    assert_eq!(
        (status, body["error"]["code"].as_str()),
        (StatusCode::TOO_MANY_REQUESTS, Some("rate_limited"))
    );

    let (status, me) = app.get("/v1/me", token(&first)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["user"]["id"], first["user"]["id"]);

    sqlx::query("update email_challenges set created_at = now() - interval '2 minutes'")
        .execute(&pool)
        .await
        .unwrap();
    let again = app.email_sign_in("mina@example.com").await;
    assert_eq!(again["user"]["id"], first["user"]["id"]);
    assert_eq!(providers(&again), ["email"]);
}

#[sqlx::test(migrator = "malmoon_auth::MIGRATOR")]
async fn email_code_is_single_use_and_attempts_are_limited(pool: PgPool) {
    let app = App::start(pool.clone()).await;
    let challenge = app.email_start("mina@example.com", None).await;
    let code = app.last_code("mina@example.com");
    let wrong = if code == "000000" { "000001" } else { "000000" };
    let verify = |code: String| {
        let body = json!({ "challenge_id": challenge, "code": code, "client": "desktop" });
        async { app.post("/v1/auth/email/verify", None, body).await }
    };

    for _ in 0..5 {
        let (status, body) = verify(wrong.to_owned()).await;
        assert_eq!(
            (status, body["error"]["code"].as_str()),
            (StatusCode::BAD_REQUEST, Some("invalid_code"))
        );
    }
    // The right code no longer helps: the guesses are used up.
    let (status, body) = verify(code).await;
    assert_eq!(
        (status, body["error"]["code"].as_str()),
        (StatusCode::TOO_MANY_REQUESTS, Some("too_many_attempts"))
    );

    // A fresh challenge works once and only once.
    sqlx::query("update email_challenges set created_at = now() - interval '2 minutes'")
        .execute(&pool)
        .await
        .unwrap();
    let challenge = app.email_start("mina@example.com", None).await;
    let body = json!({ "challenge_id": challenge, "code": app.last_code("mina@example.com"), "client": "desktop" });
    assert_eq!(
        app.post("/v1/auth/email/verify", None, body.clone())
            .await
            .0,
        StatusCode::OK
    );
    let (status, again) = app.post("/v1/auth/email/verify", None, body).await;
    assert_eq!(
        (status, again["error"]["code"].as_str()),
        (StatusCode::BAD_REQUEST, Some("invalid_code"))
    );
}

#[sqlx::test(migrator = "malmoon_auth::MIGRATOR")]
async fn email_start_rejects_non_addresses_and_caps_codes_per_hour(pool: PgPool) {
    let app = App::start(pool.clone()).await;
    let (status, body) = app
        .post(
            "/v1/auth/email/start",
            None,
            json!({ "email": "not-an-address" }),
        )
        .await;
    assert_eq!(
        (status, body["error"]["code"].as_str()),
        (StatusCode::BAD_REQUEST, Some("invalid_request"))
    );

    for _ in 0..5 {
        app.email_start("mina@example.com", None).await;
        sqlx::query("update email_challenges set created_at = created_at - interval '2 minutes'")
            .execute(&pool)
            .await
            .unwrap();
    }
    let (status, body) = app
        .post(
            "/v1/auth/email/start",
            None,
            json!({ "email": "mina@example.com" }),
        )
        .await;
    assert_eq!(
        (status, body["error"]["code"].as_str()),
        (StatusCode::TOO_MANY_REQUESTS, Some("rate_limited"))
    );
}

// ---------------------------------------------------------------- social providers

#[sqlx::test(migrator = "malmoon_auth::MIGRATOR")]
async fn provider_sign_in_creates_an_account_once(pool: PgPool) {
    let app = App::start(pool).await;
    let first = app.provider_sign_in("google", &MINA).await;
    assert_eq!(providers(&first), ["google"]);
    assert!(token(&first).starts_with("mms_"));

    let second = app.provider_sign_in("google", &MINA).await;
    assert_eq!(second["user"]["id"], first["user"]["id"]);
    assert_ne!(token(&second), token(&first));

    // The provider was asked with its secret and a PKCE verifier of our own.
    let requests = app.idp.token_requests.lock().unwrap();
    assert_eq!(requests[0]["client_secret"], "provider-secret");
    assert_eq!(requests[0]["client_id"], "client-google");
    assert!(requests[0].contains_key("code_verifier"));
}

#[sqlx::test(migrator = "malmoon_auth::MIGRATOR")]
async fn a_verified_address_joins_the_account_that_already_owns_it(pool: PgPool) {
    let app = App::start(pool).await;
    let by_email = app.email_sign_in("mina@example.com").await;
    let by_google = app.provider_sign_in("google", &MINA).await;
    assert_eq!(by_google["user"]["id"], by_email["user"]["id"]);
    assert_eq!(providers(&by_google), ["email", "google"]);
}

#[sqlx::test(migrator = "malmoon_auth::MIGRATOR")]
async fn an_unverified_address_cannot_take_over_an_account(pool: PgPool) {
    let app = App::start(pool).await;
    let victim = app.email_sign_in("mina@example.com").await;

    // LINE reports the address but does not vouch for it.
    let impostor = Person {
        sub: "line-impostor",
        email: Some("mina@example.com"),
        verified: None,
    };
    let redirect = app.provider_redirect("line", &impostor, None).await;
    assert_eq!(query(&redirect, "error").as_deref(), Some("email_in_use"));
    assert_eq!(query(&redirect, "code"), None);

    let (_, me) = app.get("/v1/me", token(&victim)).await;
    assert_eq!(providers(&me), ["email"]);
}

#[sqlx::test(migrator = "malmoon_auth::MIGRATOR")]
async fn an_unverified_first_account_is_not_merged_into_later(pool: PgPool) {
    let app = App::start(pool).await;
    // Someone registers the address at a provider that does not check it...
    let squatter = Person {
        sub: "line-squatter",
        email: Some("mina@example.com"),
        verified: None,
    };
    let squatted = app.provider_sign_in("line", &squatter).await;
    // ...and the real owner, proving the mailbox, gets an account of their own.
    let owner = app.email_sign_in("mina@example.com").await;
    assert_ne!(owner["user"]["id"], squatted["user"]["id"]);
    assert_eq!(providers(&owner), ["email"]);
}

#[sqlx::test(migrator = "malmoon_auth::MIGRATOR")]
async fn userinfo_can_vouch_for_an_address_the_id_token_does_not(pool: PgPool) {
    let app = App::start(pool).await;
    let owner = app.email_sign_in("mina@example.com").await;
    let kakao = Person {
        sub: "kakao-mina",
        email: Some("mina@example.com"),
        verified: None,
    };

    // Without a userinfo answer the address counts as unverified.
    let redirect = app.provider_redirect("kakao", &kakao, None).await;
    assert_eq!(query(&redirect, "error").as_deref(), Some("email_in_use"));

    let next_code = format!(
        "code-{}",
        app.codes.load(std::sync::atomic::Ordering::Relaxed)
    );
    app.idp.userinfo.lock().unwrap().insert(
        format!("access-{next_code}"),
        json!({ "sub": "kakao-mina", "email": "mina@example.com", "email_verified": true }),
    );
    let signed_in = app.provider_sign_in("kakao", &kakao).await;
    assert_eq!(signed_in["user"]["id"], owner["user"]["id"]);
    assert_eq!(providers(&signed_in), ["email", "kakao"]);
}

#[sqlx::test(migrator = "malmoon_auth::MIGRATOR")]
async fn linking_adds_a_provider_and_unlinking_keeps_one(pool: PgPool) {
    let app = App::start(pool).await;
    let mina = app.provider_sign_in("google", &MINA).await;
    let line = Person {
        sub: "line-mina",
        email: None,
        verified: None,
    };

    // Linking needs a session.
    let body = json!({ "client": "desktop", "redirect_uri": DESKTOP_REDIRECT, "code_challenge": pkce_challenge(VERIFIER), "intent": "link" });
    let (status, _) = app.post("/v1/auth/oauth/line/start", None, body).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let redirect = app
        .provider_redirect("line", &line, Some(token(&mina)))
        .await;
    assert_eq!(query(&redirect, "linked").as_deref(), Some("line"));
    assert_eq!(query(&redirect, "code"), None);
    // From now on LINE signs in to the same account.
    let by_line = app.provider_sign_in("line", &line).await;
    assert_eq!(by_line["user"]["id"], mina["user"]["id"]);
    assert_eq!(providers(&by_line), ["google", "line"]);

    // An email address can be linked the same way.
    let challenge = app
        .email_start("mina@work.example", Some(token(&mina)))
        .await;
    let body = json!({ "challenge_id": challenge, "code": app.last_code("mina@work.example"), "client": "desktop" });
    let (status, linked) = app
        .post("/v1/auth/email/verify", Some(token(&mina)), body)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(linked["session"], Value::Null);
    assert_eq!(providers(&linked), ["google", "line", "email"]);

    // Someone else cannot link an identity that is already Mina's.
    let other = app.email_sign_in("other@example.com").await;
    let redirect = app
        .provider_redirect("line", &line, Some(token(&other)))
        .await;
    assert_eq!(
        query(&redirect, "error").as_deref(),
        Some("identity_in_use")
    );

    // Unlink down to one method, and no further.
    let unlink = |identity: &Value, bearer: String| {
        let url = app.url(&format!(
            "/v1/me/identities/{}",
            identity["id"].as_str().unwrap()
        ));
        async {
            app.http
                .delete(url)
                .bearer_auth(bearer)
                .send()
                .await
                .unwrap()
        }
    };
    let identities = linked["identities"].as_array().unwrap();
    assert_eq!(
        unlink(&identities[0], token(&other).to_owned())
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        unlink(&identities[0], token(&mina).to_owned())
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        unlink(&identities[1], token(&mina).to_owned())
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    let last = unlink(&identities[2], token(&mina).to_owned()).await;
    assert_eq!(last.status(), StatusCode::CONFLICT);
    assert_eq!(
        last.json::<Value>().await.unwrap()["error"]["code"],
        "last_identity"
    );
}

// ---------------------------------------------------------------- the flow's own defenses

#[sqlx::test(migrator = "malmoon_auth::MIGRATOR")]
async fn state_and_login_code_work_once_and_need_the_verifier(pool: PgPool) {
    let app = App::start(pool).await;

    // A login code is useless without the verifier of the app that started the flow, and the
    // failed attempt spends it.
    let redirect = app.provider_redirect("google", &MINA, None).await;
    let code = query(&redirect, "code").unwrap();
    let (status, body) = app
        .post(
            "/v1/auth/token",
            None,
            json!({ "code": code, "code_verifier": "someone-else" }),
        )
        .await;
    assert_eq!(
        (status, body["error"]["code"].as_str()),
        (StatusCode::BAD_REQUEST, Some("flow_expired"))
    );
    let (status, _) = app
        .post(
            "/v1/auth/token",
            None,
            json!({ "code": code, "code_verifier": VERIFIER }),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // A state value cannot be replayed, and an unknown one gets a page, not a redirect.
    assert!(
        app.callback("google", "code-x", "never-issued")
            .await
            .is_none()
    );
    let body = json!({ "client": "desktop", "redirect_uri": DESKTOP_REDIRECT, "code_challenge": pkce_challenge(VERIFIER) });
    let (_, started) = app.post("/v1/auth/oauth/google/start", None, body).await;
    let authorize = Url::parse(started["authorization_url"].as_str().unwrap()).unwrap();
    let state = query(&authorize, "state").unwrap();
    // The provider answers with a token for a different attempt (wrong nonce): refused.
    app.idp.id_tokens.lock().unwrap().insert(
        "stolen".to_owned(),
        json!({ "iss": app.idp_base, "aud": "client-google", "exp": 4_000_000_000u64, "sub": "google-mina", "nonce": "other" }),
    );
    let redirect = app.callback("google", "stolen", &state).await.unwrap();
    assert_eq!(query(&redirect, "error").as_deref(), Some("provider_error"));
    assert!(app.callback("google", "stolen", &state).await.is_none());
}

#[sqlx::test(migrator = "malmoon_auth::MIGRATOR")]
async fn start_refuses_bad_redirects_challenges_and_providers(pool: PgPool) {
    let app = &App::start(pool).await;
    let start = |provider: &'static str,
                 client: &'static str,
                 redirect_uri: &'static str,
                 challenge: String| {
        let body =
            json!({ "client": client, "redirect_uri": redirect_uri, "code_challenge": challenge });
        async move {
            app.post(&format!("/v1/auth/oauth/{provider}/start"), None, body)
                .await
        }
    };
    let challenge = pkce_challenge(VERIFIER);

    assert_eq!(
        start("google", "desktop", DESKTOP_REDIRECT, challenge.clone())
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        start("google", "web", WEB_REDIRECT, challenge.clone())
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        start(
            "google",
            "mobile",
            "malmoon://auth/callback",
            challenge.clone()
        )
        .await
        .0,
        StatusCode::OK
    );
    // A redirect that is fine for one client is not fine for another.
    assert_eq!(
        start(
            "google",
            "web",
            "https://evil.example/cb",
            challenge.clone()
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        start("google", "desktop", WEB_REDIRECT, challenge.clone())
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        start("google", "mobile", DESKTOP_REDIRECT, challenge.clone())
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        start(
            "google",
            "desktop",
            DESKTOP_REDIRECT,
            "too-short".to_owned()
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    // Apple is not configured in this test; email is not a social provider.
    let (status, body) = start("apple", "desktop", DESKTOP_REDIRECT, challenge.clone()).await;
    assert_eq!(
        (status, body["error"]["code"].as_str()),
        (StatusCode::NOT_FOUND, Some("provider_unavailable"))
    );
    assert_eq!(
        start("email", "desktop", DESKTOP_REDIRECT, challenge)
            .await
            .0,
        StatusCode::BAD_REQUEST
    );

    let (_, listed) = app.get("/v1/auth/providers", "").await;
    assert_eq!(
        listed["providers"],
        json!(["email", "google", "kakao", "line"])
    );
}

// ---------------------------------------------------------------- sessions

#[sqlx::test(migrator = "malmoon_auth::MIGRATOR")]
async fn web_sessions_live_in_a_cookie_and_check_the_origin(pool: PgPool) {
    let app = App::start(pool).await;
    let challenge = app.email_start("mina@example.com", None).await;
    let body = json!({ "challenge_id": challenge, "code": app.last_code("mina@example.com"), "client": "web" });
    let verify = app.http.post(app.url("/v1/auth/email/verify")).json(&body);

    // Another site cannot sign a browser in, and the refused attempt does not spend the code.
    let refused = verify
        .try_clone()
        .unwrap()
        .header(header::ORIGIN, "https://evil.example")
        .send()
        .await
        .unwrap();
    assert_eq!(refused.status(), StatusCode::FORBIDDEN);

    let response = verify
        .header(header::ORIGIN, WEB_ORIGIN)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let set_cookie = response.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .to_owned();
    assert!(
        set_cookie.contains("HttpOnly") && set_cookie.contains("SameSite=Lax"),
        "{set_cookie}"
    );
    // The token is in the cookie only: scripts on the page never see it.
    assert_eq!(
        response.json::<Value>().await.unwrap()["session"]["token"],
        Value::Null
    );
    let cookie = set_cookie.split(';').next().unwrap().to_owned();

    let me = app
        .http
        .get(app.url("/v1/me"))
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(me.status(), StatusCode::OK);

    // A cookie alone does not authorize a change: the request must come from the web app.
    let logout = || {
        app.http
            .post(app.url("/v1/auth/logout"))
            .header(header::COOKIE, &cookie)
    };
    assert_eq!(
        logout().send().await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        logout()
            .header(header::ORIGIN, "https://evil.example")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let response = logout()
        .header(header::ORIGIN, WEB_ORIGIN)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(
        response.headers()[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .contains("Max-Age=0")
    );

    let me = app
        .http
        .get(app.url("/v1/me"))
        .header(header::COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(me.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "malmoon_auth::MIGRATOR")]
async fn sessions_end_on_sign_out_and_on_expiry(pool: PgPool) {
    let app = App::start(pool.clone()).await;
    let first = app.email_sign_in("mina@example.com").await;
    sqlx::query("update email_challenges set created_at = now() - interval '2 minutes'")
        .execute(&pool)
        .await
        .unwrap();
    let second = app.email_sign_in("mina@example.com").await;

    assert_eq!(
        app.get("/v1/me", "mms_not-a-session").await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.post("/v1/auth/logout", Some(token(&first)), json!({}))
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        app.get("/v1/me", token(&first)).await.0,
        StatusCode::UNAUTHORIZED
    );
    // Signing out one device leaves the other signed in.
    assert_eq!(app.get("/v1/me", token(&second)).await.0, StatusCode::OK);

    sqlx::query("update sessions set expires_at = now() - interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        app.get("/v1/me", token(&second)).await.0,
        StatusCode::UNAUTHORIZED
    );

    // Only hashes of tokens are stored.
    let stored: Vec<Vec<u8>> = sqlx::query_scalar("select token_hash from sessions")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(
        stored
            .iter()
            .all(|hash| hash.len() == 32 && hash.as_slice() != token(&second).as_bytes())
    );
}

#[sqlx::test(migrator = "malmoon_auth::MIGRATOR")]
async fn a_web_session_can_sign_the_desktop_app_in(pool: PgPool) {
    let app = App::start(pool).await;
    let challenge = app.email_start("mina@example.com", None).await;
    let body = json!({ "challenge_id": challenge, "code": app.last_code("mina@example.com"), "client": "web" });
    let response = app
        .http
        .post(app.url("/v1/auth/email/verify"))
        .header(header::ORIGIN, WEB_ORIGIN)
        .json(&body)
        .send()
        .await
        .unwrap();
    let cookie = response.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let web_user = response.json::<Value>().await.unwrap()["user"]["id"].clone();

    let handoff = |redirect_uri: &'static str, origin: Option<&'static str>| {
        let body = json!({ "redirect_uri": redirect_uri, "code_challenge": pkce_challenge(VERIFIER), "state": "desktop-state" });
        let mut request = app
            .http
            .post(app.url("/v1/auth/handoff"))
            .header(header::COOKIE, &cookie)
            .json(&body);
        if let Some(origin) = origin {
            request = request.header(header::ORIGIN, origin);
        }
        async { request.send().await.unwrap() }
    };
    // Only the web app may ask, and only for a loopback address.
    assert_eq!(
        handoff(DESKTOP_REDIRECT, None).await.status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        handoff("https://evil.example/callback", Some(WEB_ORIGIN))
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );

    let response = handoff(DESKTOP_REDIRECT, Some(WEB_ORIGIN)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let redirect = Url::parse(
        response.json::<Value>().await.unwrap()["redirect_url"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert!(redirect.as_str().starts_with(DESKTOP_REDIRECT));
    assert_eq!(query(&redirect, "state").as_deref(), Some("desktop-state"));

    // The desktop app trades the code for a session of its own, for the same user.
    let code = query(&redirect, "code").unwrap();
    let (status, desktop) = app
        .post(
            "/v1/auth/token",
            None,
            json!({ "code": code, "code_verifier": VERIFIER }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(desktop["user"]["id"], web_user);
    assert_eq!(app.get("/v1/me", token(&desktop)).await.0, StatusCode::OK);
}
