//! Every SQL statement of the account system.

use malmoon_core::protocol::ClientKind;
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    error::AuthError,
    model::{
        Account, Identity, Provider, ProviderProfile, SessionInfo, User, client_str,
        normalize_email, parse_client,
    },
    token,
};

pub(crate) const SESSION_DAYS: i64 = 30;
/// A session's expiry moves forward when it is used, at most this often.
const SESSION_TOUCH_SECONDS: i64 = 3600;

// ---------------------------------------------------------------- accounts

/// Signs a person in with what a provider said about them, and returns their user id.
///
/// - The identity is known: that is its user. Nothing else is considered.
/// - `link_to` is set: a signed-in user is adding this identity to their own account.
/// - Otherwise the identity is new. If the provider vouches for the address and exactly one
///   account already holds that address as verified, the identity joins that account: both
///   sides proved control of the same mailbox. If the address is taken but the provider does not
///   vouch for it, sign-in is refused, because anyone can claim an address they do not own.
///   With no match, a new account is created.
pub(crate) async fn sign_in(
    conn: &mut PgConnection,
    profile: &ProviderProfile,
    link_to: Option<Uuid>,
) -> Result<Uuid, AuthError> {
    let email = profile.email.as_deref().and_then(normalize_email);
    let existing: Option<(Uuid, Uuid)> = sqlx::query_as(
        "select id, user_id from identities where provider = $1 and subject = $2 for update",
    )
    .bind(profile.provider.as_str())
    .bind(&profile.subject)
    .fetch_optional(&mut *conn)
    .await?;

    if let Some((identity, owner)) = existing {
        if link_to.is_some_and(|user| user != owner) {
            return Err(AuthError::IdentityInUse);
        }
        sqlx::query(
            "update identities set last_login_at = now(), email = $2, email_verified = $3,
                    display_name = coalesce($4, display_name)
              where id = $1",
        )
        .bind(identity)
        .bind(&email)
        .bind(profile.email_verified && email.is_some())
        .bind(&profile.display_name)
        .execute(&mut *conn)
        .await?;
        return Ok(owner);
    }

    let user = match link_to {
        Some(user) => user,
        None => {
            let owners: Vec<Uuid> = match &email {
                Some(email) => {
                    sqlx::query_scalar(
                        "select distinct user_id from identities where email = $1 and email_verified limit 2",
                    )
                    .bind(email)
                    .fetch_all(&mut *conn)
                    .await?
                }
                None => Vec::new(),
            };
            match owners.as_slice() {
                [] => {
                    let user = Uuid::now_v7();
                    sqlx::query("insert into users (id, display_name) values ($1, $2)")
                        .bind(user)
                        .bind(&profile.display_name)
                        .execute(&mut *conn)
                        .await?;
                    user
                }
                [owner] if profile.email_verified => *owner,
                // Unverified, or two accounts hold the address and the choice is not ours.
                _ => return Err(AuthError::EmailInUse),
            }
        }
    };

    sqlx::query(
        "insert into identities (id, user_id, provider, subject, email, email_verified, display_name)
         values ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(Uuid::now_v7())
    .bind(user)
    .bind(profile.provider.as_str())
    .bind(&profile.subject)
    .bind(&email)
    .bind(profile.email_verified && email.is_some())
    .bind(&profile.display_name)
    .execute(&mut *conn)
    .await?;
    Ok(user)
}

pub(crate) async fn account(pool: &PgPool, user_id: Uuid) -> Result<Account, AuthError> {
    let user: User = sqlx::query_as("select id, display_name, created_at from users where id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .ok_or(AuthError::Unauthorized)?;
    let identities: Vec<Identity> = sqlx::query_as(
        "select id, provider, email, email_verified, display_name, created_at, last_login_at
           from identities where user_id = $1 order by created_at, id",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(Account { user, identities })
}

/// Removes one sign-in method, never the last: that would lock the person out.
pub(crate) async fn unlink(
    pool: &PgPool,
    user_id: Uuid,
    identity_id: Uuid,
) -> Result<(), AuthError> {
    let mut tx = pool.begin().await?;
    // Serializes concurrent unlinks of the same account, so two cannot each leave "one other".
    sqlx::query("select 1 from users where id = $1 for update")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    let identities: Vec<Uuid> = sqlx::query_scalar("select id from identities where user_id = $1")
        .bind(user_id)
        .fetch_all(&mut *tx)
        .await?;
    if !identities.contains(&identity_id) {
        return Err(AuthError::InvalidRequest(
            "no such sign-in method on this account".to_owned(),
        ));
    }
    if identities.len() == 1 {
        return Err(AuthError::LastIdentity);
    }
    sqlx::query("delete from identities where id = $1")
        .bind(identity_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

// ---------------------------------------------------------------- sessions

pub(crate) struct NewSession {
    pub token: String,
    pub expires_at: OffsetDateTime,
}

pub(crate) async fn create_session(
    pool: &PgPool,
    user_id: Uuid,
    client: ClientKind,
    label: Option<&str>,
) -> Result<NewSession, AuthError> {
    sqlx::query("delete from sessions where user_id = $1 and expires_at < now()")
        .bind(user_id)
        .execute(pool)
        .await?;
    let token = token::new_session_token();
    let expires_at: OffsetDateTime = sqlx::query_scalar(
        "insert into sessions (id, user_id, token_hash, client, label, expires_at)
         values ($1, $2, $3, $4, $5, now() + make_interval(days => $6))
         returning expires_at",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(token::hash(&token))
    .bind(client_str(client))
    .bind(label)
    .bind(SESSION_DAYS as i32)
    .fetch_one(pool)
    .await?;
    Ok(NewSession { token, expires_at })
}

pub(crate) async fn session(
    pool: &PgPool,
    session_token: &str,
) -> Result<Option<SessionInfo>, AuthError> {
    let row: Option<(Uuid, Uuid, String, bool)> = sqlx::query_as(
        "select id, user_id, client, last_used_at < now() - make_interval(secs => $2)
           from sessions where token_hash = $1 and expires_at > now()",
    )
    .bind(token::hash(session_token))
    .bind(SESSION_TOUCH_SECONDS as f64)
    .fetch_optional(pool)
    .await?;
    let Some((session_id, user_id, client, stale)) = row else {
        return Ok(None);
    };
    if stale {
        sqlx::query(
            "update sessions set last_used_at = now(), expires_at = now() + make_interval(days => $2) where id = $1",
        )
        .bind(session_id)
        .bind(SESSION_DAYS as i32)
        .execute(pool)
        .await?;
    }
    Ok(Some(SessionInfo {
        session_id,
        user_id,
        client: parse_client(&client)?,
    }))
}

pub(crate) async fn delete_session(pool: &PgPool, session_id: Uuid) -> Result<(), AuthError> {
    sqlx::query("delete from sessions where id = $1")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ---------------------------------------------------------------- provider round trips

pub(crate) struct NewFlow<'a> {
    pub state: &'a str,
    pub provider: Provider,
    pub nonce: &'a str,
    pub pkce_verifier: Option<&'a str>,
    pub client: ClientKind,
    pub redirect_uri: &'a str,
    pub app_state: Option<&'a str>,
    pub code_challenge: &'a str,
    pub link_user_id: Option<Uuid>,
    pub ttl_seconds: i64,
}

#[derive(sqlx::FromRow)]
pub(crate) struct Flow {
    pub nonce: String,
    pub pkce_verifier: Option<String>,
    pub client: String,
    pub redirect_uri: String,
    pub app_state: Option<String>,
    pub code_challenge: String,
    pub link_user_id: Option<Uuid>,
}

pub(crate) async fn insert_flow(pool: &PgPool, flow: NewFlow<'_>) -> Result<(), AuthError> {
    // Abandoned attempts are never read again; sweep them when a new one starts.
    sqlx::query("delete from oauth_flows where expires_at < now()")
        .execute(pool)
        .await?;
    sqlx::query("delete from login_codes where expires_at < now()")
        .execute(pool)
        .await?;
    sqlx::query(
        "insert into oauth_flows (state_hash, provider, nonce, pkce_verifier, client, redirect_uri,
                                  app_state, code_challenge, link_user_id, expires_at)
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9, now() + make_interval(secs => $10))",
    )
    .bind(token::hash(flow.state))
    .bind(flow.provider.as_str())
    .bind(flow.nonce)
    .bind(flow.pkce_verifier)
    .bind(client_str(flow.client))
    .bind(flow.redirect_uri)
    .bind(flow.app_state)
    .bind(flow.code_challenge)
    .bind(flow.link_user_id)
    .bind(flow.ttl_seconds as f64)
    .execute(pool)
    .await?;
    Ok(())
}

/// Removes the flow as it reads it, so a `state` value works once.
pub(crate) async fn take_flow(
    pool: &PgPool,
    provider: Provider,
    state: &str,
) -> Result<Option<Flow>, AuthError> {
    let flow = sqlx::query_as(
        "delete from oauth_flows where state_hash = $1 and provider = $2 and expires_at > now()
         returning nonce, pkce_verifier, client, redirect_uri, app_state, code_challenge, link_user_id",
    )
    .bind(token::hash(state))
    .bind(provider.as_str())
    .fetch_optional(pool)
    .await?;
    Ok(flow)
}

pub(crate) async fn insert_login_code(
    pool: &PgPool,
    code: &str,
    user_id: Uuid,
    client: &str,
    code_challenge: &str,
    ttl_seconds: i64,
) -> Result<(), AuthError> {
    sqlx::query(
        "insert into login_codes (code_hash, user_id, client, code_challenge, expires_at)
         values ($1, $2, $3, $4, now() + make_interval(secs => $5))",
    )
    .bind(token::hash(code))
    .bind(user_id)
    .bind(client)
    .bind(code_challenge)
    .bind(ttl_seconds as f64)
    .execute(pool)
    .await?;
    Ok(())
}

/// Removes the code as it reads it, so it works once even if the PKCE check then fails.
pub(crate) async fn take_login_code(
    pool: &PgPool,
    code: &str,
) -> Result<Option<(Uuid, String, String)>, AuthError> {
    let row = sqlx::query_as(
        "delete from login_codes where code_hash = $1 and expires_at > now()
         returning user_id, client, code_challenge",
    )
    .bind(token::hash(code))
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

// ---------------------------------------------------------------- email codes

pub(crate) struct RecentChallenges {
    pub count: i64,
    pub oldest: Option<OffsetDateTime>,
    pub newest: Option<OffsetDateTime>,
}

pub(crate) async fn recent_challenges(
    pool: &PgPool,
    email: &str,
) -> Result<RecentChallenges, AuthError> {
    sqlx::query("delete from email_challenges where created_at < now() - interval '1 day'")
        .execute(pool)
        .await?;
    let (count, oldest, newest) = sqlx::query_as(
        "select count(*), min(created_at), max(created_at) from email_challenges
          where email = $1 and created_at > now() - interval '1 hour'",
    )
    .bind(email)
    .fetch_one(pool)
    .await?;
    Ok(RecentChallenges {
        count,
        oldest,
        newest,
    })
}

pub(crate) async fn insert_challenge(
    pool: &PgPool,
    id: Uuid,
    email: &str,
    code: &str,
    link_user_id: Option<Uuid>,
    ttl_seconds: i64,
) -> Result<(), AuthError> {
    sqlx::query(
        "insert into email_challenges (id, email, code_hash, link_user_id, expires_at)
         values ($1, $2, $3, $4, now() + make_interval(secs => $5))",
    )
    .bind(id)
    .bind(email)
    .bind(token::hash_email_code(id, code))
    .bind(link_user_id)
    .bind(ttl_seconds as f64)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn delete_challenge(pool: &PgPool, id: Uuid) -> Result<(), AuthError> {
    sqlx::query("delete from email_challenges where id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
pub(crate) struct ChallengeAttempt {
    pub email: String,
    pub code_hash: Vec<u8>,
    pub attempts: i32,
    pub link_user_id: Option<Uuid>,
}

/// Counts the attempt before anything is compared, and outside any transaction, so that a wrong
/// guess is recorded no matter what happens next.
pub(crate) async fn count_attempt(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<ChallengeAttempt>, AuthError> {
    let attempt = sqlx::query_as(
        "update email_challenges set attempts = attempts + 1
          where id = $1 and consumed_at is null and expires_at > now()
         returning email, code_hash, attempts, link_user_id",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(attempt)
}

/// False when another request consumed the challenge first.
pub(crate) async fn consume_challenge(
    conn: &mut PgConnection,
    id: Uuid,
) -> Result<bool, AuthError> {
    let result = sqlx::query(
        "update email_challenges set consumed_at = now() where id = $1 and consumed_at is null",
    )
    .bind(id)
    .execute(conn)
    .await?;
    Ok(result.rows_affected() == 1)
}
