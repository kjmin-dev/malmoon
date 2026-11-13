//! Delivery of sign-in codes.

use std::{future::Future, pin::Pin};

pub type MailFuture<'a> = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;

pub trait Mailer: Send + Sync + 'static {
    fn send_login_code<'a>(&'a self, to: &'a str, code: &'a str) -> MailFuture<'a>;
}

/// Writes the code to the log instead of sending it. For development only: anyone who can read
/// the log can sign in as anyone.
#[derive(Debug, Default)]
pub struct LogMailer;

impl Mailer for LogMailer {
    fn send_login_code<'a>(&'a self, to: &'a str, code: &'a str) -> MailFuture<'a> {
        Box::pin(async move {
            tracing::warn!(
                to,
                code,
                "sign-in code (not sent: no mail provider is configured)"
            );
            Ok(())
        })
    }
}
