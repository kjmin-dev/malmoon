use std::collections::HashMap;

use anyhow::{Context, bail};
use malmoon_core::protocol::ClientKind;
use url::Url;

use crate::{error::AuthError, model::Provider, provider::ProviderConfig};

#[derive(Clone, Debug)]
pub struct AuthConfig {
    /// How browsers reach this server, without a trailing slash. The providers send people back
    /// to `{public_url}/v1/auth/oauth/{provider}/callback`, so that URL is what gets registered
    /// with each provider.
    pub public_url: String,
    /// Origins of the web app. They may use cookie sessions, and browsers may be sent back to
    /// them after a provider sign-in.
    pub web_origins: Vec<String>,
    /// Redirect URIs of the mobile app, matched exactly.
    pub app_redirects: Vec<String>,
    pub providers: HashMap<Provider, ProviderConfig>,
}

impl AuthConfig {
    /// For development: the web app on Vite's port, the mobile app's scheme, no providers.
    pub fn local() -> Self {
        Self {
            public_url: "http://localhost:8787".to_owned(),
            web_origins: vec!["http://localhost:5173".to_owned()],
            app_redirects: vec!["malmoon://auth/callback".to_owned()],
            providers: HashMap::new(),
        }
    }

    /// Reads `MALMOON_PUBLIC_URL`, `MALMOON_WEB_ORIGINS`, `MALMOON_APP_REDIRECTS` (comma
    /// separated) and the provider credentials. A provider is on when its client id is set.
    pub fn from_env() -> anyhow::Result<Self> {
        let var = |name: &str| {
            std::env::var(name)
                .ok()
                .filter(|value| !value.trim().is_empty())
        };
        let list = |value: String| {
            value
                .split(',')
                .map(|item| item.trim().to_owned())
                .collect::<Vec<_>>()
        };
        let require = |name: &str| {
            var(name).with_context(|| {
                format!("{name} must be set with the other credentials of its provider")
            })
        };

        let mut config = Self::local();
        if let Some(url) = var("MALMOON_PUBLIC_URL") {
            config.public_url = url.trim_end_matches('/').to_owned();
        }
        if let Some(origins) = var("MALMOON_WEB_ORIGINS") {
            config.web_origins = list(origins);
        }
        if let Some(redirects) = var("MALMOON_APP_REDIRECTS") {
            config.app_redirects = list(redirects);
        }
        for origin in &config.web_origins {
            let parsed = Url::parse(origin)
                .with_context(|| format!("web origin {origin:?} is not a URL"))?;
            if parsed.origin().ascii_serialization() != *origin {
                bail!("web origin {origin:?} must be scheme://host[:port] with no path");
            }
        }

        if let Some(id) = var("MALMOON_GOOGLE_CLIENT_ID") {
            let secret = require("MALMOON_GOOGLE_CLIENT_SECRET")?;
            config
                .providers
                .insert(Provider::Google, ProviderConfig::google(id, secret));
        }
        if let Some(id) = var("MALMOON_KAKAO_CLIENT_ID") {
            let secret = var("MALMOON_KAKAO_CLIENT_SECRET");
            config
                .providers
                .insert(Provider::Kakao, ProviderConfig::kakao(id, secret));
        }
        if let Some(id) = var("MALMOON_LINE_CLIENT_ID") {
            let secret = require("MALMOON_LINE_CLIENT_SECRET")?;
            config
                .providers
                .insert(Provider::Line, ProviderConfig::line(id, secret));
        }
        if let Some(id) = var("MALMOON_APPLE_CLIENT_ID") {
            let apple = ProviderConfig::apple(
                id,
                require("MALMOON_APPLE_TEAM_ID")?,
                require("MALMOON_APPLE_KEY_ID")?,
                // Environment files often hold the key on one line with literal \n.
                require("MALMOON_APPLE_PRIVATE_KEY")?.replace("\\n", "\n"),
            );
            config.providers.insert(Provider::Apple, apple);
        }
        Ok(config)
    }

    pub(crate) fn callback_url(&self, provider: Provider) -> String {
        format!("{}/v1/auth/oauth/{provider}/callback", self.public_url)
    }

    pub(crate) fn secure_cookies(&self) -> bool {
        self.public_url.starts_with("https://")
    }

    pub(crate) fn is_web_origin(&self, origin: &str) -> bool {
        self.web_origins.iter().any(|allowed| allowed == origin)
    }

    /// Decides whether the browser may be sent to `redirect_uri` with a login code. An open
    /// redirect here would hand codes to whoever asked.
    ///
    /// - Web: any path on an allowed web origin.
    /// - Desktop: the loopback interface on any port (RFC 8252, section 7.3). Only a program on
    ///   the same machine can listen there.
    /// - Mobile: one of the registered app redirects, exactly.
    pub(crate) fn check_redirect(
        &self,
        client: ClientKind,
        redirect_uri: &str,
    ) -> Result<(), AuthError> {
        let refuse = || {
            AuthError::InvalidRequest(format!(
                "redirect_uri {redirect_uri:?} is not allowed for this client"
            ))
        };
        let url = Url::parse(redirect_uri).map_err(|_| refuse())?;
        if url.fragment().is_some() || !url.username().is_empty() || url.password().is_some() {
            return Err(refuse());
        }
        let allowed = match client {
            ClientKind::Web => self.is_web_origin(&url.origin().ascii_serialization()),
            ClientKind::Desktop => {
                url.scheme() == "http"
                    && matches!(url.host_str(), Some("127.0.0.1" | "[::1]"))
                    && url.port().is_some()
                    && url.path() == "/callback"
                    && url.query().is_none()
            }
            ClientKind::Mobile => self
                .app_redirects
                .iter()
                .any(|allowed| allowed == redirect_uri),
        };
        if allowed { Ok(()) } else { Err(refuse()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_redirects_must_stay_on_an_allowed_origin() {
        let config = AuthConfig::local();
        config
            .check_redirect(ClientKind::Web, "http://localhost:5173/auth/callback")
            .unwrap();
        for uri in [
            "http://localhost:5174/auth/callback",
            "https://localhost:5173/auth/callback",
            "http://localhost:5173.evil.example/",
            "http://evil.example/?http://localhost:5173",
            "http://user@localhost:5173/",
            "javascript:alert(1)",
            "not a url",
        ] {
            assert!(
                config.check_redirect(ClientKind::Web, uri).is_err(),
                "{uri}"
            );
        }
    }

    #[test]
    fn desktop_redirects_must_be_loopback() {
        let config = AuthConfig::local();
        config
            .check_redirect(ClientKind::Desktop, "http://127.0.0.1:49152/callback")
            .unwrap();
        config
            .check_redirect(ClientKind::Desktop, "http://[::1]:49152/callback")
            .unwrap();
        for uri in [
            "http://127.0.0.1/callback",
            "http://localhost:49152/callback",
            "http://127.0.0.1.evil.example:49152/callback",
            "https://127.0.0.1:49152/callback",
            "http://127.0.0.1:49152/other",
            "http://192.168.0.2:49152/callback",
        ] {
            assert!(
                config.check_redirect(ClientKind::Desktop, uri).is_err(),
                "{uri}"
            );
        }
    }

    #[test]
    fn mobile_redirects_must_match_exactly() {
        let config = AuthConfig::local();
        config
            .check_redirect(ClientKind::Mobile, "malmoon://auth/callback")
            .unwrap();
        for uri in [
            "malmoon://auth/callback/extra",
            "malmoon://other",
            "evil://auth/callback",
            "http://localhost:5173/",
        ] {
            assert!(
                config.check_redirect(ClientKind::Mobile, uri).is_err(),
                "{uri}"
            );
        }
    }
}
