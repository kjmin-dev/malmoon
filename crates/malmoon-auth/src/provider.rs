//! The OpenID Connect authorization-code flow against one social provider.
//!
//! This server is a confidential client: it sends the browser to the provider, receives a code
//! on its own callback, and trades the code for an ID token directly with the provider's token
//! endpoint. Because the token arrives over that TLS connection and never passes through the
//! browser, its signature is not checked (OpenID Connect Core 3.1.3.7 allows this). Its issuer,
//! audience, expiry and nonce are. Accepting an ID token from an app, as native Sign in with
//! Apple does, would need signature verification against the provider's keys.

use std::time::Duration;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;
use time::OffsetDateTime;
use url::Url;

use crate::{
    error::AuthError,
    model::{Provider, ProviderProfile},
    token::pkce_challenge,
};

/// Clock difference tolerated when checking an ID token's expiry.
const CLOCK_SKEW_SECONDS: i64 = 60;

#[derive(Clone, Debug)]
pub struct ProviderConfig {
    pub client_id: String,
    pub secret: ClientSecret,
    pub authorize_url: String,
    pub token_url: String,
    /// Asked for `email_verified` when the ID token does not carry it.
    pub userinfo_url: Option<String>,
    pub issuers: Vec<String>,
    pub scope: String,
    /// Send a PKCE challenge to the provider as well. Off where support is not documented.
    pub pkce: bool,
    /// Apple posts the callback as a form when a name or email is requested.
    pub form_post: bool,
    /// Apple only hands out addresses it has verified.
    pub email_always_verified: bool,
}

#[derive(Clone)]
pub enum ClientSecret {
    None,
    Static(String),
    /// Apple's client secret is a short-lived JWT signed with the team's key.
    AppleKey {
        team_id: String,
        key_id: String,
        private_key_pem: String,
    },
}

impl std::fmt::Debug for ClientSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ClientSecret::None => "ClientSecret::None",
            ClientSecret::Static(_) => "ClientSecret::Static(..)",
            ClientSecret::AppleKey { .. } => "ClientSecret::AppleKey(..)",
        })
    }
}

impl ProviderConfig {
    pub fn google(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            secret: ClientSecret::Static(client_secret),
            authorize_url: "https://accounts.google.com/o/oauth2/v2/auth".to_owned(),
            token_url: "https://oauth2.googleapis.com/token".to_owned(),
            userinfo_url: None,
            issuers: vec![
                "https://accounts.google.com".to_owned(),
                "accounts.google.com".to_owned(),
            ],
            scope: "openid email profile".to_owned(),
            pkce: true,
            form_post: false,
            email_always_verified: false,
        }
    }

    /// `client_id` is the Services ID, not the app's bundle id.
    pub fn apple(
        client_id: String,
        team_id: String,
        key_id: String,
        private_key_pem: String,
    ) -> Self {
        Self {
            client_id,
            secret: ClientSecret::AppleKey {
                team_id,
                key_id,
                private_key_pem,
            },
            authorize_url: "https://appleid.apple.com/auth/authorize".to_owned(),
            token_url: "https://appleid.apple.com/auth/token".to_owned(),
            userinfo_url: None,
            issuers: vec![APPLE_ISSUER.to_owned()],
            scope: "openid name email".to_owned(),
            pkce: false,
            form_post: true,
            email_always_verified: true,
        }
    }

    /// `client_id` is the REST API key. OpenID Connect must be switched on for the app in the
    /// Kakao console, and the secret is needed unless it was switched off there.
    pub fn kakao(client_id: String, client_secret: Option<String>) -> Self {
        Self {
            client_id,
            secret: client_secret.map_or(ClientSecret::None, ClientSecret::Static),
            authorize_url: "https://kauth.kakao.com/oauth/authorize".to_owned(),
            token_url: "https://kauth.kakao.com/oauth/token".to_owned(),
            // Kakao's ID token has no verified flag for the address.
            userinfo_url: Some("https://kapi.kakao.com/v1/oidc/userinfo".to_owned()),
            issuers: vec!["https://kauth.kakao.com".to_owned()],
            scope: "openid account_email profile_nickname".to_owned(),
            pkce: true,
            form_post: false,
            email_always_verified: false,
        }
    }

    /// `client_id` is the channel id. LINE does not say whether the address it reports is
    /// verified, so it is treated as unverified.
    pub fn line(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            secret: ClientSecret::Static(client_secret),
            authorize_url: "https://access.line.me/oauth2/v2.1/authorize".to_owned(),
            token_url: "https://api.line.me/oauth2/v2.1/token".to_owned(),
            userinfo_url: None,
            issuers: vec!["https://access.line.me".to_owned()],
            scope: "openid profile email".to_owned(),
            pkce: true,
            form_post: false,
            email_always_verified: false,
        }
    }

    /// The URL to send the browser to.
    pub(crate) fn authorization_url(
        &self,
        redirect_uri: &str,
        state: &str,
        nonce: &str,
        pkce_verifier: Option<&str>,
    ) -> Result<String, AuthError> {
        let mut url = Url::parse(&self.authorize_url).map_err(|error| {
            anyhow::anyhow!("bad authorize URL {}: {error}", self.authorize_url)
        })?;
        {
            let mut query = url.query_pairs_mut();
            query
                .append_pair("response_type", "code")
                .append_pair("client_id", &self.client_id)
                .append_pair("redirect_uri", redirect_uri)
                .append_pair("scope", &self.scope)
                .append_pair("state", state)
                .append_pair("nonce", nonce);
            if let Some(verifier) = pkce_verifier {
                query
                    .append_pair("code_challenge", &pkce_challenge(verifier))
                    .append_pair("code_challenge_method", "S256");
            }
            if self.form_post {
                query.append_pair("response_mode", "form_post");
            }
        }
        Ok(url.into())
    }

    /// Trades the authorization code for the person's profile.
    pub(crate) async fn exchange(
        &self,
        http: &reqwest::Client,
        provider: Provider,
        code: &str,
        redirect_uri: &str,
        nonce: &str,
        pkce_verifier: Option<&str>,
    ) -> Result<ProviderProfile, AuthError> {
        let secret = match &self.secret {
            ClientSecret::None => None,
            ClientSecret::Static(secret) => Some(secret.clone()),
            ClientSecret::AppleKey {
                team_id,
                key_id,
                private_key_pem,
            } => Some(apple_client_secret(
                &self.client_id,
                team_id,
                key_id,
                private_key_pem,
                OffsetDateTime::now_utc(),
            )?),
        };
        let mut form = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", &self.client_id),
        ];
        if let Some(secret) = &secret {
            form.push(("client_secret", secret));
        }
        if let Some(verifier) = pkce_verifier {
            form.push(("code_verifier", verifier));
        }

        let response = http
            .post(&self.token_url)
            .timeout(Duration::from_secs(10))
            .form(&form)
            .send()
            .await
            .map_err(|error| AuthError::Provider(format!("token request failed: {error}")))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| AuthError::Provider(format!("token response unreadable: {error}")))?;
        if !status.is_success() {
            return Err(AuthError::Provider(format!(
                "token endpoint answered {status}: {body}"
            )));
        }
        let tokens: TokenResponse = serde_json::from_str(&body).map_err(|error| {
            AuthError::Provider(format!("token response has no ID token: {error}"))
        })?;

        let claims = decode_claims(&tokens.id_token)?;
        self.validate(&claims, nonce, OffsetDateTime::now_utc().unix_timestamp())?;

        let mut email_verified =
            self.email_always_verified || claims.email_verified.as_ref().is_some_and(Flag::is_true);
        if claims.email.is_some()
            && claims.email_verified.is_none()
            && !email_verified
            && let (Some(url), Some(access_token)) = (&self.userinfo_url, &tokens.access_token)
        {
            email_verified = userinfo_says_verified(http, url, access_token, &claims).await;
        }

        Ok(ProviderProfile {
            provider,
            subject: claims.sub,
            email: claims.email,
            email_verified,
            display_name: claims.name.or(claims.nickname),
        })
    }

    fn validate(&self, claims: &IdClaims, nonce: &str, now: i64) -> Result<(), AuthError> {
        let reject = |why: &str| Err(AuthError::Provider(format!("ID token rejected: {why}")));
        if !self.issuers.contains(&claims.iss) {
            return reject("unexpected issuer");
        }
        let for_us = match &claims.aud {
            Audience::One(audience) => audience == &self.client_id,
            Audience::Many(audiences) => audiences.contains(&self.client_id),
        };
        if !for_us {
            return reject("issued to a different client");
        }
        if claims.exp + CLOCK_SKEW_SECONDS < now {
            return reject("expired");
        }
        if claims.nonce.as_deref() != Some(nonce) {
            return reject("nonce does not match this sign-in attempt");
        }
        if claims.sub.is_empty() {
            return reject("no subject");
        }
        Ok(())
    }
}

const APPLE_ISSUER: &str = "https://appleid.apple.com";

#[derive(Deserialize)]
struct TokenResponse {
    id_token: String,
    access_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IdClaims {
    iss: String,
    aud: Audience,
    exp: i64,
    sub: String,
    nonce: Option<String>,
    email: Option<String>,
    email_verified: Option<Flag>,
    name: Option<String>,
    nickname: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Audience {
    One(String),
    Many(Vec<String>),
}

/// Apple has sent `email_verified` both as a boolean and as the string "true".
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Flag {
    Bool(bool),
    Text(String),
}

impl Flag {
    fn is_true(&self) -> bool {
        match self {
            Flag::Bool(value) => *value,
            Flag::Text(value) => value == "true",
        }
    }
}

fn decode_claims(id_token: &str) -> Result<IdClaims, AuthError> {
    let malformed = |why: String| AuthError::Provider(format!("ID token malformed: {why}"));
    let mut parts = id_token.split('.');
    let (Some(_header), Some(payload), Some(_signature), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(malformed("not three segments".to_owned()));
    };
    let bytes = URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .map_err(|error| malformed(error.to_string()))?;
    serde_json::from_slice(&bytes).map_err(|error| malformed(error.to_string()))
}

#[derive(Deserialize)]
struct UserInfo {
    sub: Option<String>,
    email: Option<String>,
    email_verified: Option<Flag>,
}

/// False on any doubt: a failed request, a different person, or a different address.
async fn userinfo_says_verified(
    http: &reqwest::Client,
    url: &str,
    access_token: &str,
    claims: &IdClaims,
) -> bool {
    let request = http
        .get(url)
        .timeout(Duration::from_secs(10))
        .bearer_auth(access_token);
    let info: UserInfo = match request
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
    {
        Ok(response) => match response.json().await {
            Ok(info) => info,
            Err(error) => {
                tracing::warn!(%error, "userinfo response unreadable; treating the email as unverified");
                return false;
            }
        },
        Err(error) => {
            tracing::warn!(%error, "userinfo request failed; treating the email as unverified");
            return false;
        }
    };
    info.sub.as_deref() == Some(claims.sub.as_str())
        && info.email == claims.email
        && info.email_verified.as_ref().is_some_and(Flag::is_true)
}

/// Apple's client secret: an ES256 JWT that says which team vouches for which client.
fn apple_client_secret(
    client_id: &str,
    team_id: &str,
    key_id: &str,
    private_key_pem: &str,
    now: OffsetDateTime,
) -> Result<String, AuthError> {
    #[derive(serde::Serialize)]
    struct Claims<'a> {
        iss: &'a str,
        iat: i64,
        exp: i64,
        aud: &'a str,
        sub: &'a str,
    }
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::ES256);
    header.kid = Some(key_id.to_owned());
    let claims = Claims {
        iss: team_id,
        iat: now.unix_timestamp(),
        // Signed per request, so it only has to outlive the request.
        exp: now.unix_timestamp() + 300,
        aud: APPLE_ISSUER,
        sub: client_id,
    };
    let key =
        jsonwebtoken::EncodingKey::from_ec_pem(private_key_pem.as_bytes()).map_err(|error| {
            anyhow::anyhow!("the Apple private key is not a valid EC key in PEM: {error}")
        })?;
    let token = jsonwebtoken::encode(&header, &claims, &key)
        .map_err(|error| anyhow::anyhow!("could not sign the Apple client secret: {error}"))?;
    Ok(token)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn id_token(claims: serde_json::Value) -> String {
        let segment = |value: &serde_json::Value| URL_SAFE_NO_PAD.encode(value.to_string());
        format!(
            "{}.{}.sig",
            segment(&json!({ "alg": "RS256" })),
            segment(&claims)
        )
    }

    fn google() -> ProviderConfig {
        ProviderConfig::google("client-1".to_owned(), "secret".to_owned())
    }

    fn claims(overrides: serde_json::Value) -> IdClaims {
        let mut claims = json!({
            "iss": "https://accounts.google.com",
            "aud": "client-1",
            "exp": 2_000,
            "sub": "subject-1",
            "nonce": "nonce-1",
        });
        claims
            .as_object_mut()
            .unwrap()
            .extend(overrides.as_object().unwrap().clone());
        decode_claims(&id_token(claims)).unwrap()
    }

    #[test]
    fn accepts_a_token_issued_for_this_attempt() {
        google()
            .validate(&claims(json!({})), "nonce-1", 1_000)
            .unwrap();
        // Audience may be a list, and the expiry tolerates a little clock skew.
        google()
            .validate(
                &claims(json!({ "aud": ["other", "client-1"] })),
                "nonce-1",
                2_030,
            )
            .unwrap();
    }

    #[test]
    fn rejects_tokens_meant_for_someone_or_something_else() {
        let cases = [
            (json!({ "iss": "https://evil.example" }), "nonce-1", 1_000),
            (json!({ "aud": "other-client" }), "nonce-1", 1_000),
            (json!({}), "another-nonce", 1_000),
            (json!({ "nonce": null }), "nonce-1", 1_000),
            (json!({}), "nonce-1", 2_100),
            (json!({ "sub": "" }), "nonce-1", 1_000),
        ];
        for (overrides, nonce, now) in cases {
            let result = google().validate(&claims(overrides.clone()), nonce, now);
            assert!(matches!(result, Err(AuthError::Provider(_))), "{overrides}");
        }
    }

    #[test]
    fn reads_the_verified_flag_as_boolean_or_string() {
        assert!(
            claims(json!({ "email_verified": true }))
                .email_verified
                .unwrap()
                .is_true()
        );
        assert!(
            claims(json!({ "email_verified": "true" }))
                .email_verified
                .unwrap()
                .is_true()
        );
        assert!(
            !claims(json!({ "email_verified": "false" }))
                .email_verified
                .unwrap()
                .is_true()
        );
        assert!(claims(json!({})).email_verified.is_none());
    }

    #[test]
    fn rejects_garbage_instead_of_panicking() {
        for token in ["", "a.b", "a.b.c.d", "a.!!!.c", "a.e30.c"] {
            assert!(decode_claims(token).is_err(), "{token:?}");
        }
    }

    #[test]
    fn authorization_url_carries_the_flow_parameters() {
        let url = google()
            .authorization_url(
                "https://api.example/cb",
                "state-1",
                "nonce-1",
                Some("verifier"),
            )
            .unwrap();
        let url = Url::parse(&url).unwrap();
        let get = |name: &str| {
            url.query_pairs()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.into_owned())
        };
        assert_eq!(
            get("redirect_uri").as_deref(),
            Some("https://api.example/cb")
        );
        assert_eq!(get("state").as_deref(), Some("state-1"));
        assert_eq!(get("nonce").as_deref(), Some("nonce-1"));
        assert_eq!(get("code_challenge_method").as_deref(), Some("S256"));
        assert_eq!(get("response_mode"), None);

        let apple = ProviderConfig::apple("svc".into(), "team".into(), "key".into(), String::new());
        let url = Url::parse(
            &apple
                .authorization_url("https://api.example/cb", "s", "n", None)
                .unwrap(),
        )
        .unwrap();
        assert!(
            url.query_pairs()
                .any(|(key, value)| key == "response_mode" && value == "form_post")
        );
        assert!(!url.query_pairs().any(|(key, _)| key == "code_challenge"));
    }

    // A throwaway P-256 key made for this test with `openssl ecparam -genkey`. It signs nothing
    // outside this file.
    const TEST_EC_KEY: &str = include_str!("../tests/fixtures/test-ec-key.pem");

    #[test]
    fn apple_client_secret_names_the_team_key_and_client() {
        let now = OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap();
        let secret = apple_client_secret(
            "com.example.web",
            "TEAM123456",
            "KEY1234567",
            TEST_EC_KEY,
            now,
        )
        .unwrap();
        let mut parts = secret.split('.');
        let decode = |segment: &str| -> serde_json::Value {
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(segment).unwrap()).unwrap()
        };
        let header = decode(parts.next().unwrap());
        let claims = decode(parts.next().unwrap());
        assert_eq!(header["alg"], "ES256");
        assert_eq!(header["kid"], "KEY1234567");
        assert_eq!(claims["iss"], "TEAM123456");
        assert_eq!(claims["sub"], "com.example.web");
        assert_eq!(claims["aud"], "https://appleid.apple.com");
        assert_eq!(
            claims["exp"].as_i64().unwrap() - claims["iat"].as_i64().unwrap(),
            300
        );
    }

    #[test]
    fn a_bad_apple_key_is_an_error_not_a_panic() {
        let now = OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap();
        assert!(apple_client_secret("c", "t", "k", "not a key", now).is_err());
    }
}
