//! Signing the desktop app in.
//!
//! The app has no sign-in form of its own. It opens the web app in the system browser, where the
//! person signs in with any method the web app offers and confirms. The browser is then sent to
//! a loopback address this app listens on, carrying a one-time login code (RFC 8252). PKCE makes
//! that code useless to anything but this process: only it knows the verifier.

use std::{path::PathBuf, time::Duration};

use anyhow::{Context as _, anyhow, bail};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use url::Url;

/// How long the person has to finish in the browser.
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(300);

fn random_secret() -> String {
    let mut bytes = [0u8; 32];
    rand::fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Opens the browser, waits for the person to sign in there, and returns a session token.
/// `gateway` and `web` are the HTTP base URLs of the gateway and the web app.
pub async fn sign_in(gateway: &str, web: &str) -> anyhow::Result<String> {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .context("could not listen for the sign-in redirect")?;
    let redirect_uri = format!(
        "http://127.0.0.1:{}/callback",
        listener.local_addr()?.port()
    );
    let verifier = random_secret();
    let state = random_secret();
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));

    let mut page = Url::parse(&format!("{}/auth/desktop", web.trim_end_matches('/')))
        .with_context(|| format!("the web app URL {web:?} is not a URL"))?;
    page.query_pairs_mut()
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("code_challenge", &challenge)
        .append_pair("state", &state);
    open_browser(page.as_str())?;

    let code = tokio::time::timeout(SIGN_IN_TIMEOUT, wait_for_redirect(&listener, &state))
        .await
        .map_err(|_| anyhow!("로그인 시간이 초과되었습니다. 다시 시도해 주세요."))??;

    #[derive(Deserialize)]
    struct Answer {
        session: Session,
    }
    #[derive(Deserialize)]
    struct Session {
        token: String,
    }
    let response = reqwest::Client::new()
        .post(format!("{gateway}/v1/auth/token"))
        .timeout(Duration::from_secs(10))
        .json(&serde_json::json!({ "code": code, "code_verifier": verifier }))
        .send()
        .await
        .context("could not reach the gateway to finish signing in")?;
    if !response.status().is_success() {
        bail!("the gateway refused the login code ({})", response.status());
    }
    Ok(response
        .json::<Answer>()
        .await
        .context("unexpected answer to the login code")?
        .session
        .token)
}

/// Opens the URL in the program named by `BROWSER` when it is set, as other command-line tools
/// do, and in the system's default browser otherwise.
fn open_browser(url: &str) -> anyhow::Result<()> {
    match std::env::var_os("BROWSER").filter(|browser| !browser.is_empty()) {
        Some(browser) => std::process::Command::new(&browser)
            .arg(url)
            .spawn()
            .map(drop)
            .with_context(|| format!("could not run BROWSER ({})", browser.display())),
        None => open::that_detached(url).context("could not open the browser"),
    }
}

/// Ends the session on the gateway. The local token is dropped whether or not this succeeds.
pub async fn sign_out(gateway: &str, token: &str) {
    let request = reqwest::Client::new()
        .post(format!("{gateway}/v1/auth/logout"))
        .timeout(Duration::from_secs(5))
        .bearer_auth(token)
        .json(&serde_json::json!({}));
    if let Err(error) = request.send().await {
        eprintln!("could not end the session on the gateway: {error}");
    }
}

/// Serves the loopback address until the browser arrives with the answer to `state`.
async fn wait_for_redirect(listener: &TcpListener, state: &str) -> anyhow::Result<String> {
    loop {
        let (mut stream, _) = listener.accept().await?;
        let Some(target) = read_request_target(&mut stream).await else {
            continue;
        };
        let Ok(url) = Url::parse(&format!("http://127.0.0.1{target}")) else {
            continue;
        };
        let param = |name: &str| {
            url.query_pairs()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.into_owned())
        };
        // Browsers also ask for /favicon.ico, and other local programs may poke the port.
        if url.path() != "/callback" || param("state").as_deref() != Some(state) {
            respond(
                &mut stream,
                "404 Not Found",
                "이 주소는 Malmoon 로그인 전용입니다.",
            )
            .await;
            continue;
        }
        return match param("code") {
            Some(code) => {
                respond(
                    &mut stream,
                    "200 OK",
                    "로그인했습니다. 이 창을 닫고 Malmoon으로 돌아가세요.",
                )
                .await;
                Ok(code)
            }
            None => {
                respond(
                    &mut stream,
                    "400 Bad Request",
                    "로그인하지 못했습니다. Malmoon에서 다시 시도해 주세요.",
                )
                .await;
                Err(anyhow!("the browser came back without a login code"))
            }
        };
    }
}

/// The path and query of the request line, for example `/callback?code=...`.
async fn read_request_target(stream: &mut TcpStream) -> Option<String> {
    let mut buffer = Vec::with_capacity(2048);
    let mut chunk = [0u8; 1024];
    while !buffer.windows(2).any(|pair| pair == b"\r\n") && buffer.len() < 8192 {
        let read = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut chunk))
            .await
            .ok()?
            .ok()?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
    }
    let line = String::from_utf8_lossy(&buffer);
    let mut parts = line.lines().next()?.split(' ');
    (parts.next()? == "GET").then(|| parts.next().map(str::to_owned))?
}

async fn respond(stream: &mut TcpStream, status: &str, message: &str) {
    let body = format!(
        "<!doctype html><html lang=\"ko\"><meta charset=\"utf-8\"><title>Malmoon</title>\
         <body style=\"font-family:system-ui;display:grid;place-items:center;height:100vh;margin:0\">\
         <p>{message}</p></body></html>"
    );
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
}

/// Where the session token is kept between runs.
///
/// Release builds use the operating system's keychain. Debug builds use a file readable only by
/// the user, because macOS ties keychain access to the binary's signature and would ask for the
/// login password after every rebuild. `MALMOON_TOKEN_STORE=keychain|file` overrides either.
pub enum TokenStore {
    Keychain { account: String },
    File(PathBuf),
}

const KEYCHAIN_SERVICE: &str = "com.malmoon.desktop";

impl TokenStore {
    /// One token per gateway, so a development gateway and the real one do not share a session.
    pub fn for_gateway(gateway: &str) -> Self {
        let use_keychain = match std::env::var("MALMOON_TOKEN_STORE").as_deref() {
            Ok("keychain") => true,
            Ok("file") => false,
            _ => !cfg!(debug_assertions),
        };
        if use_keychain {
            return TokenStore::Keychain {
                account: gateway.to_owned(),
            };
        }
        let name: String = gateway
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        TokenStore::File(data_dir().join(format!("session-{name}.token")))
    }

    pub fn load(&self) -> Option<String> {
        let token = match self {
            TokenStore::Keychain { account } => keyring::Entry::new(KEYCHAIN_SERVICE, account)
                .ok()?
                .get_password()
                .ok()?,
            TokenStore::File(path) => std::fs::read_to_string(path).ok()?,
        };
        let token = token.trim();
        (!token.is_empty()).then(|| token.to_owned())
    }

    pub fn save(&self, token: &str) -> anyhow::Result<()> {
        match self {
            TokenStore::Keychain { account } => keyring::Entry::new(KEYCHAIN_SERVICE, account)
                .and_then(|entry| entry.set_password(token))
                .context("could not save the session to the keychain"),
            TokenStore::File(path) => write_private(path, token)
                .with_context(|| format!("could not save the session to {}", path.display())),
        }
    }

    pub fn clear(&self) {
        match self {
            TokenStore::Keychain { account } => {
                if let Ok(entry) = keyring::Entry::new(KEYCHAIN_SERVICE, account) {
                    let _ = entry.delete_credential();
                }
            }
            TokenStore::File(path) => {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

fn data_dir() -> PathBuf {
    let home = || PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    if cfg!(target_os = "macos") {
        home().join("Library/Application Support/Malmoon")
    } else if cfg!(windows) {
        PathBuf::from(std::env::var_os("APPDATA").unwrap_or_default()).join("Malmoon")
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map_or_else(|| home().join(".local/share"), PathBuf::from)
            .join("malmoon")
    }
}

fn write_private(path: &std::path::Path, contents: &str) -> std::io::Result<()> {
    use std::io::Write as _;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    options.open(path)?.write_all(contents.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn get(port: u16, target: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        let request = format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        response
    }

    #[tokio::test]
    async fn only_the_answer_to_this_sign_in_is_accepted() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let waiting = tokio::spawn(async move { wait_for_redirect(&listener, "state-1").await });

        // A favicon request, another program's guess, and a stale redirect do not end the wait.
        assert!(get(port, "/favicon.ico").await.starts_with("HTTP/1.1 404"));
        assert!(
            get(port, "/callback?code=stolen")
                .await
                .starts_with("HTTP/1.1 404")
        );
        assert!(
            get(port, "/callback?code=stolen&state=other")
                .await
                .starts_with("HTTP/1.1 404")
        );
        assert!(!waiting.is_finished());

        let page = get(port, "/callback?code=login-code&state=state-1").await;
        assert!(
            page.starts_with("HTTP/1.1 200") && page.contains("로그인했습니다"),
            "{page}"
        );
        assert_eq!(waiting.await.unwrap().unwrap(), "login-code");
    }

    #[tokio::test]
    async fn a_redirect_without_a_code_ends_the_attempt() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let waiting = tokio::spawn(async move { wait_for_redirect(&listener, "state-1").await });
        assert!(
            get(port, "/callback?state=state-1")
                .await
                .starts_with("HTTP/1.1 400")
        );
        assert!(waiting.await.unwrap().is_err());
    }

    #[test]
    fn the_token_file_round_trips_and_is_private() {
        let path = std::env::temp_dir()
            .join(format!("malmoon-test-{}", random_secret()))
            .join("session.token");
        let store = TokenStore::File(path.clone());
        assert_eq!(store.load(), None);
        store.save("mms_token").unwrap();
        assert_eq!(store.load().as_deref(), Some("mms_token"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        store.clear();
        assert_eq!(store.load(), None);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
