//! Random secrets and their hashes.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// 256 random bits, URL-safe. Used for session tokens, OAuth state, nonces and login codes.
pub(crate) fn random_secret() -> String {
    let mut bytes = [0u8; 32];
    rand::fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Session tokens carry a prefix so that a leaked one is recognizable in logs and scanners.
pub(crate) fn new_session_token() -> String {
    format!("mms_{}", random_secret())
}

/// The secrets above are high-entropy, so a plain hash is enough to store them safely.
pub(crate) fn hash(secret: &str) -> Vec<u8> {
    Sha256::digest(secret.as_bytes()).to_vec()
}

/// The S256 code challenge of RFC 7636 for a verifier.
pub fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// A six-digit code, uniformly distributed, zero-padded.
pub(crate) fn new_email_code() -> String {
    format!("{:06}", rand::random_range(0..1_000_000u32))
}

/// Six digits are guessable offline, so the hash binds the code to its challenge. The attempt
/// limit and the ten-minute lifetime are what actually protect it.
pub(crate) fn hash_email_code(challenge: Uuid, code: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(challenge.as_bytes());
    hasher.update(code.as_bytes());
    hasher.finalize().to_vec()
}

pub(crate) fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .fold(0u8, |diff, (a, b)| diff | (a ^ b))
            == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_matches_the_rfc_example() {
        // RFC 7636, appendix B.
        assert_eq!(
            pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn secrets_are_unique_and_url_safe() {
        let first = random_secret();
        assert_ne!(first, random_secret());
        assert_eq!(first.len(), 43);
        assert!(
            first
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
    }

    #[test]
    fn email_codes_are_six_digits() {
        for _ in 0..100 {
            let code = new_email_code();
            assert!(
                code.len() == 6 && code.chars().all(|c| c.is_ascii_digit()),
                "{code}"
            );
        }
    }

    #[test]
    fn code_hash_depends_on_the_challenge() {
        let (first, second) = (Uuid::now_v7(), Uuid::now_v7());
        assert_ne!(
            hash_email_code(first, "123456"),
            hash_email_code(second, "123456")
        );
        assert!(constant_time_eq(
            &hash_email_code(first, "123456"),
            &hash_email_code(first, "123456")
        ));
    }
}
