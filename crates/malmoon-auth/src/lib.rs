//! Accounts for Malmoon: who a person is, how they sign in, and which sessions they hold.
//!
//! A user owns one or more identities. An identity is one way to sign in: a one-time code sent
//! to an email address, or a social provider (Google, Apple, Kakao, LINE). Sessions are opaque
//! random tokens; only their hash is stored.
//!
//! Every client signs in the same way. It asks this server to start a provider sign-in, opens
//! the returned URL in a browser, and is sent back with a one-time login code that it trades
//! for a session. The provider's tokens never leave this server. `docs/07-accounts.md` has the
//! full design.

#![forbid(unsafe_code)]

mod config;
mod error;
mod http;
mod mail;
mod model;
mod provider;
mod store;
mod token;

use std::sync::Arc;

use anyhow::Context;
use malmoon_core::protocol::ClientKind;
use sqlx::{PgPool, postgres::PgPoolOptions};
use time::OffsetDateTime;
use url::Url;
use uuid::Uuid;

pub use crate::{
    config::AuthConfig,
    error::AuthError,
    http::{CurrentSession, SESSION_COOKIE},
    mail::{LogMailer, MailFuture, Mailer},
    model::{Account, Identity, Provider, ProviderProfile, SessionInfo, User, normalize_email},
    provider::{ClientSecret, ProviderConfig},
    token::pkce_challenge,
};

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

/// How long a person has to come back from the provider.
const FLOW_SECONDS: i64 = 600;
/// How long the app has to trade its login code. It does so immediately.
const LOGIN_CODE_SECONDS: i64 = 60;
pub const EMAIL_CODE_SECONDS: i64 = 600;
pub const EMAIL_RESEND_SECONDS: i64 = 60;
const EMAIL_CODES_PER_HOUR: i64 = 5;
const EMAIL_MAX_ATTEMPTS: i32 = 5;

/// The account system. Cheap to clone.
#[derive(Clone)]
pub struct Auth {
    inner: Arc<Inner>,
}

struct Inner {
    pool: PgPool,
    config: AuthConfig,
    http: reqwest::Client,
    mailer: Box<dyn Mailer>,
}

pub struct OAuthStart<'a> {
    pub provider: Provider,
    pub client: ClientKind,
    pub redirect_uri: &'a str,
    /// The S256 challenge of a verifier only the app knows.
    pub code_challenge: &'a str,
    /// Returned to the app untouched on its redirect URI.
    pub state: Option<&'a str>,
    /// The signed-in user who is adding this provider, when linking.
    pub link_user: Option<Uuid>,
}

pub struct EmailChallenge {
    pub challenge_id: Uuid,
}

/// A new session and the account it belongs to.
pub struct SignedIn {
    pub account: Account,
    pub client: ClientKind,
    pub token: String,
    pub expires_at: OffsetDateTime,
}

pub enum EmailVerified {
    SignedIn(SignedIn),
    /// The address was added to the account that asked; its session is unchanged.
    Linked(Account),
}

impl Auth {
    /// Connects to the database and brings its schema up to date.
    pub async fn connect(
        database_url: &str,
        config: AuthConfig,
        mailer: impl Mailer,
    ) -> anyhow::Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await
            .context("could not connect to the database")?;
        MIGRATOR
            .run(&pool)
            .await
            .context("could not migrate the database")?;
        Ok(Self::new(pool, config, mailer))
    }

    /// Uses a pool whose schema is already current.
    pub fn new(pool: PgPool, config: AuthConfig, mailer: impl Mailer) -> Self {
        let http = reqwest::Client::builder()
            // A provider has no reason to redirect a token request.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("the HTTP client has a static configuration");
        Self {
            inner: Arc::new(Inner {
                pool,
                config,
                http,
                mailer: Box::new(mailer),
            }),
        }
    }

    pub fn config(&self) -> &AuthConfig {
        &self.inner.config
    }

    /// The sign-in methods this server offers, email first.
    pub fn providers(&self) -> Vec<Provider> {
        let social = Provider::SOCIAL
            .into_iter()
            .filter(|provider| self.inner.config.providers.contains_key(provider));
        std::iter::once(Provider::Email).chain(social).collect()
    }

    /// The session a token belongs to, if it is still valid.
    pub async fn authenticate(
        &self,
        session_token: &str,
    ) -> Result<Option<SessionInfo>, AuthError> {
        store::session(&self.inner.pool, session_token).await
    }

    pub async fn account(&self, user_id: Uuid) -> Result<Account, AuthError> {
        store::account(&self.inner.pool, user_id).await
    }

    pub async fn unlink(&self, user_id: Uuid, identity_id: Uuid) -> Result<(), AuthError> {
        store::unlink(&self.inner.pool, user_id, identity_id).await
    }

    pub async fn sign_out(&self, session_id: Uuid) -> Result<(), AuthError> {
        store::delete_session(&self.inner.pool, session_id).await
    }

    // ------------------------------------------------------------ social providers

    /// Begins a provider sign-in and returns the URL to open in a browser.
    pub async fn start_oauth(&self, start: OAuthStart<'_>) -> Result<String, AuthError> {
        let config = &self.inner.config;
        let provider = config
            .providers
            .get(&start.provider)
            .ok_or(AuthError::ProviderUnavailable(start.provider))?;
        config.check_redirect(start.client, start.redirect_uri)?;
        check_challenge(start.code_challenge)?;
        if start.state.is_some_and(|state| state.len() > 512) {
            return Err(AuthError::InvalidRequest(
                "state is longer than 512 bytes".to_owned(),
            ));
        }

        let state = token::random_secret();
        let nonce = token::random_secret();
        let pkce_verifier = provider.pkce.then(token::random_secret);
        store::insert_flow(
            &self.inner.pool,
            store::NewFlow {
                state: &state,
                provider: start.provider,
                nonce: &nonce,
                pkce_verifier: pkce_verifier.as_deref(),
                client: start.client,
                redirect_uri: start.redirect_uri,
                app_state: start.state,
                code_challenge: start.code_challenge,
                link_user_id: start.link_user,
                ttl_seconds: FLOW_SECONDS,
            },
        )
        .await?;
        provider.authorization_url(
            &config.callback_url(start.provider),
            &state,
            &nonce,
            pkce_verifier.as_deref(),
        )
    }

    /// Handles the provider's callback and returns where to send the browser: the app's redirect
    /// URI with a login code, `linked`, or `error`. Fails only when there is no app to return
    /// to, because the attempt is unknown, expired or already used.
    pub(crate) async fn finish_oauth(
        &self,
        provider: Provider,
        callback: http::Callback,
    ) -> Result<String, AuthError> {
        let state = callback.state.as_deref().ok_or(AuthError::FlowExpired)?;
        let flow = store::take_flow(&self.inner.pool, provider, state)
            .await?
            .ok_or(AuthError::FlowExpired)?;
        let mut redirect = Url::parse(&flow.redirect_uri)
            .map_err(|error| anyhow::anyhow!("stored redirect URI does not parse: {error}"))?;

        let outcome = self.complete_flow(provider, &flow, &callback).await;
        {
            let mut query = redirect.query_pairs_mut();
            match &outcome {
                Ok(Some(code)) => query.append_pair("code", code),
                Ok(None) => query.append_pair("linked", provider.as_str()),
                Err(error) => {
                    match error {
                        AuthError::Internal(cause) => {
                            tracing::error!(%provider, error = format!("{cause:#}"), "sign-in failed")
                        }
                        other => tracing::info!(%provider, error = %other, "sign-in not completed"),
                    }
                    query.append_pair("error", error.code())
                }
            };
            if let Some(app_state) = &flow.app_state {
                query.append_pair("state", app_state);
            }
        }
        Ok(redirect.into())
    }

    /// `Some(login code)` after a sign-in, `None` after linking.
    async fn complete_flow(
        &self,
        provider: Provider,
        flow: &store::Flow,
        callback: &http::Callback,
    ) -> Result<Option<String>, AuthError> {
        if let Some(error) = &callback.error {
            return Err(match error.as_str() {
                "access_denied" | "user_cancelled_authorize" => AuthError::Cancelled,
                other => AuthError::Provider(format!("the provider returned {other}")),
            });
        }
        let code = callback
            .code
            .as_deref()
            .ok_or_else(|| AuthError::Provider("the callback carried no code".to_owned()))?;
        let config = &self.inner.config;
        let provider_config = config
            .providers
            .get(&provider)
            .ok_or(AuthError::ProviderUnavailable(provider))?;
        let mut profile = provider_config
            .exchange(
                &self.inner.http,
                provider,
                code,
                &config.callback_url(provider),
                &flow.nonce,
                flow.pkce_verifier.as_deref(),
            )
            .await?;
        if profile.display_name.is_none() {
            profile.display_name = callback.user.as_deref().and_then(http::apple_display_name);
        }

        let user_id = self.sign_in(&profile, flow.link_user_id, None).await?;
        if flow.link_user_id.is_some() {
            return Ok(None);
        }
        let login_code = token::random_secret();
        store::insert_login_code(
            &self.inner.pool,
            &login_code,
            user_id,
            &flow.client,
            &flow.code_challenge,
            LOGIN_CODE_SECONDS,
        )
        .await?;
        Ok(Some(login_code))
    }

    /// Trades the login code from the redirect for a session. The verifier proves the caller is
    /// the app that started the sign-in, so a code that leaks from a redirect URL is useless.
    pub async fn exchange_login_code(
        &self,
        code: &str,
        code_verifier: &str,
        label: Option<&str>,
    ) -> Result<SignedIn, AuthError> {
        let (user_id, client, code_challenge) = store::take_login_code(&self.inner.pool, code)
            .await?
            .ok_or(AuthError::FlowExpired)?;
        if pkce_challenge(code_verifier) != code_challenge {
            return Err(AuthError::FlowExpired);
        }
        self.open_session(user_id, model::parse_client(&client)?, label)
            .await
    }

    /// Signs the desktop app in from a web session: "continue in the browser". The desktop app
    /// opens the web app with its loopback redirect URI and PKCE challenge, the person confirms
    /// there, and the browser is sent to the returned URL, which carries a login code only that
    /// desktop app can trade for a session of its own.
    pub async fn handoff(
        &self,
        user_id: Uuid,
        redirect_uri: &str,
        code_challenge: &str,
        state: Option<&str>,
    ) -> Result<String, AuthError> {
        self.inner
            .config
            .check_redirect(ClientKind::Desktop, redirect_uri)?;
        check_challenge(code_challenge)?;
        let login_code = token::random_secret();
        store::insert_login_code(
            &self.inner.pool,
            &login_code,
            user_id,
            model::client_str(ClientKind::Desktop),
            code_challenge,
            LOGIN_CODE_SECONDS,
        )
        .await?;
        let mut redirect = Url::parse(redirect_uri)
            .map_err(|error| anyhow::anyhow!("checked redirect URI does not parse: {error}"))?;
        {
            let mut query = redirect.query_pairs_mut();
            query.append_pair("code", &login_code);
            if let Some(state) = state {
                query.append_pair("state", state);
            }
        }
        Ok(redirect.into())
    }

    // ------------------------------------------------------------ email codes

    /// Sends a six-digit code to the address. The answer is the same whether or not an account
    /// exists, so this cannot be used to find out who has one.
    pub async fn start_email(
        &self,
        email: &str,
        link_user: Option<Uuid>,
    ) -> Result<EmailChallenge, AuthError> {
        let email = normalize_email(email).ok_or_else(|| {
            AuthError::InvalidRequest("that does not look like an email address".to_owned())
        })?;
        let pool = &self.inner.pool;

        let recent = store::recent_challenges(pool, &email).await?;
        let now = OffsetDateTime::now_utc();
        let wait_until = |from: Option<OffsetDateTime>, seconds: i64| {
            from.map_or(0, |from| {
                (from + time::Duration::seconds(seconds) - now).whole_seconds()
            })
        };
        let cooldown = wait_until(recent.newest, EMAIL_RESEND_SECONDS);
        if cooldown > 0 {
            return Err(AuthError::RateLimited {
                retry_after: cooldown as u64,
            });
        }
        if recent.count >= EMAIL_CODES_PER_HOUR {
            return Err(AuthError::RateLimited {
                retry_after: wait_until(recent.oldest, 3600).max(1) as u64,
            });
        }

        let challenge_id = Uuid::now_v7();
        let code = token::new_email_code();
        store::insert_challenge(
            pool,
            challenge_id,
            &email,
            &code,
            link_user,
            EMAIL_CODE_SECONDS,
        )
        .await?;
        if let Err(error) = self.inner.mailer.send_login_code(&email, &code).await {
            // Do not make the person wait out a cooldown for a code that never left.
            store::delete_challenge(pool, challenge_id).await?;
            return Err(AuthError::Internal(
                error.context("could not send the sign-in code"),
            ));
        }
        Ok(EmailChallenge { challenge_id })
    }

    pub async fn verify_email(
        &self,
        challenge_id: Uuid,
        code: &str,
        client: ClientKind,
        label: Option<&str>,
    ) -> Result<EmailVerified, AuthError> {
        let attempt = store::count_attempt(&self.inner.pool, challenge_id)
            .await?
            .ok_or(AuthError::InvalidCode)?;
        if attempt.attempts > EMAIL_MAX_ATTEMPTS {
            return Err(AuthError::TooManyAttempts);
        }
        if !token::constant_time_eq(
            &token::hash_email_code(challenge_id, code.trim()),
            &attempt.code_hash,
        ) {
            return Err(AuthError::InvalidCode);
        }
        let profile = ProviderProfile {
            provider: Provider::Email,
            subject: attempt.email.clone(),
            email: Some(attempt.email),
            // Entering the code is the proof.
            email_verified: true,
            display_name: None,
        };
        let user_id = self
            .sign_in(&profile, attempt.link_user_id, Some(challenge_id))
            .await?;
        if attempt.link_user_id.is_some() {
            return Ok(EmailVerified::Linked(self.account(user_id).await?));
        }
        Ok(EmailVerified::SignedIn(
            self.open_session(user_id, client, label).await?,
        ))
    }

    // ------------------------------------------------------------ shared

    /// Runs the sign-in rules in one transaction, together with consuming the email challenge
    /// that proved the identity. Two first sign-ins of the same new identity can race; the loser
    /// hits the unique index and is retried once, when it finds the identity the winner made.
    async fn sign_in(
        &self,
        profile: &ProviderProfile,
        link_to: Option<Uuid>,
        challenge: Option<Uuid>,
    ) -> Result<Uuid, AuthError> {
        let mut retried = false;
        loop {
            let mut tx = self.inner.pool.begin().await?;
            if let Some(challenge) = challenge
                && !store::consume_challenge(&mut tx, challenge).await?
            {
                return Err(AuthError::InvalidCode);
            }
            match store::sign_in(&mut tx, profile, link_to).await {
                Ok(user_id) => {
                    tx.commit().await?;
                    return Ok(user_id);
                }
                Err(AuthError::Internal(error)) if !retried && is_unique_violation(&error) => {
                    retried = true
                }
                Err(error) => return Err(error),
            }
        }
    }

    async fn open_session(
        &self,
        user_id: Uuid,
        client: ClientKind,
        label: Option<&str>,
    ) -> Result<SignedIn, AuthError> {
        let session = store::create_session(&self.inner.pool, user_id, client, label).await?;
        let account = self.account(user_id).await?;
        Ok(SignedIn {
            account,
            client,
            token: session.token,
            expires_at: session.expires_at,
        })
    }
}

/// An S256 challenge is 32 bytes in unpadded base64url.
fn check_challenge(code_challenge: &str) -> Result<(), AuthError> {
    let well_formed = code_challenge.len() == 43
        && code_challenge
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if well_formed {
        Ok(())
    } else {
        Err(AuthError::InvalidRequest(
            "code_challenge must be an S256 PKCE challenge".to_owned(),
        ))
    }
}

fn is_unique_violation(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<sqlx::Error>()
        .and_then(sqlx::Error::as_database_error)
        .is_some_and(|error| error.is_unique_violation())
}
