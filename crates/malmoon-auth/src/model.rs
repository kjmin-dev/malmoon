//! The account types the HTTP API returns.

use std::{fmt, str::FromStr};

use malmoon_core::protocol::ClientKind;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// A way to sign in. `Email` is a one-time code sent to the address; the rest are social
/// providers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Email,
    Google,
    Apple,
    Kakao,
    Line,
}

impl Provider {
    pub const SOCIAL: [Provider; 4] = [
        Provider::Google,
        Provider::Apple,
        Provider::Kakao,
        Provider::Line,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Provider::Email => "email",
            Provider::Google => "google",
            Provider::Apple => "apple",
            Provider::Kakao => "kakao",
            Provider::Line => "line",
        }
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Provider {
    type Err = UnknownProvider;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "email" => Ok(Provider::Email),
            "google" => Ok(Provider::Google),
            "apple" => Ok(Provider::Apple),
            "kakao" => Ok(Provider::Kakao),
            "line" => Ok(Provider::Line),
            other => Err(UnknownProvider(other.to_owned())),
        }
    }
}

impl TryFrom<String> for Provider {
    type Error = UnknownProvider;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unknown provider {0:?}")]
pub struct UnknownProvider(String);

pub(crate) fn client_str(client: ClientKind) -> &'static str {
    match client {
        ClientKind::Desktop => "desktop",
        ClientKind::Web => "web",
        ClientKind::Mobile => "mobile",
    }
}

pub(crate) fn parse_client(value: &str) -> anyhow::Result<ClientKind> {
    match value {
        "desktop" => Ok(ClientKind::Desktop),
        "web" => Ok(ClientKind::Web),
        "mobile" => Ok(ClientKind::Mobile),
        other => anyhow::bail!("unknown client kind {other:?} in the database"),
    }
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub display_name: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct Identity {
    pub id: Uuid,
    #[sqlx(try_from = "String")]
    pub provider: Provider,
    pub email: Option<String>,
    pub email_verified: bool,
    pub display_name: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub last_login_at: OffsetDateTime,
}

/// A user and every way they can sign in.
#[derive(Clone, Debug, Serialize)]
pub struct Account {
    pub user: User,
    pub identities: Vec<Identity>,
}

/// The session a request was made with.
#[derive(Clone, Copy, Debug)]
pub struct SessionInfo {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub client: ClientKind,
}

/// What a provider says about the person who just signed in.
#[derive(Clone, Debug)]
pub struct ProviderProfile {
    pub provider: Provider,
    pub subject: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub display_name: Option<String>,
}

/// Trims and lowercases an address, and rejects strings that are plainly not one. Delivery of
/// the code is the real validation.
pub fn normalize_email(input: &str) -> Option<String> {
    let email = input.trim().to_lowercase();
    let (local, domain) = email.split_once('@')?;
    let plausible = !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !domain.contains('@')
        && email.len() <= 254
        && !email.chars().any(|c| c.is_whitespace() || c.is_control());
    plausible.then_some(email)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_case_and_whitespace() {
        assert_eq!(
            normalize_email("  Mina@Example.COM "),
            Some("mina@example.com".to_owned())
        );
    }

    #[test]
    fn rejects_strings_that_are_not_addresses() {
        for input in [
            "",
            "mina",
            "@example.com",
            "mina@",
            "mina@localhost",
            "a b@example.com",
            "a@b@c.com",
            "a@.com",
        ] {
            assert_eq!(normalize_email(input), None, "{input:?}");
        }
    }

    #[test]
    fn provider_names_round_trip() {
        for provider in Provider::SOCIAL.into_iter().chain([Provider::Email]) {
            assert_eq!(provider.as_str().parse::<Provider>().unwrap(), provider);
        }
    }
}
