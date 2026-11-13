use axum::{
    Json,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::model::Provider;

/// Every way a request to the account API can fail. `code` is the stable name clients switch
/// on; the message is for logs and developers.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("{0}")]
    InvalidRequest(String),
    #[error("sign in to continue")]
    Unauthorized,
    #[error("requests with a cookie session must come from an allowed web origin")]
    ForbiddenOrigin,
    #[error("{0} sign-in is not configured on this server")]
    ProviderUnavailable(Provider),
    #[error("the sign-in attempt expired or was already used")]
    FlowExpired,
    #[error("the code is wrong or has expired")]
    InvalidCode,
    #[error("too many wrong codes; request a new one")]
    TooManyAttempts,
    #[error("wait {retry_after} seconds before requesting another code")]
    RateLimited { retry_after: u64 },
    #[error("an account already uses this email; sign in to it and link this provider there")]
    EmailInUse,
    #[error("this sign-in method is linked to another account")]
    IdentityInUse,
    #[error("the last sign-in method of an account cannot be removed")]
    LastIdentity,
    #[error("the sign-in was cancelled at the provider")]
    Cancelled,
    #[error("the provider did not complete the sign-in: {0}")]
    Provider(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl AuthError {
    pub fn code(&self) -> &'static str {
        match self {
            AuthError::InvalidRequest(_) => "invalid_request",
            AuthError::Unauthorized => "unauthorized",
            AuthError::ForbiddenOrigin => "forbidden_origin",
            AuthError::ProviderUnavailable(_) => "provider_unavailable",
            AuthError::FlowExpired => "flow_expired",
            AuthError::InvalidCode => "invalid_code",
            AuthError::TooManyAttempts => "too_many_attempts",
            AuthError::RateLimited { .. } => "rate_limited",
            AuthError::EmailInUse => "email_in_use",
            AuthError::IdentityInUse => "identity_in_use",
            AuthError::LastIdentity => "last_identity",
            AuthError::Cancelled => "cancelled",
            AuthError::Provider(_) => "provider_error",
            AuthError::Internal(_) => "internal",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            AuthError::InvalidRequest(_)
            | AuthError::FlowExpired
            | AuthError::InvalidCode
            | AuthError::Cancelled => StatusCode::BAD_REQUEST,
            AuthError::Unauthorized => StatusCode::UNAUTHORIZED,
            AuthError::ForbiddenOrigin => StatusCode::FORBIDDEN,
            AuthError::ProviderUnavailable(_) => StatusCode::NOT_FOUND,
            AuthError::TooManyAttempts | AuthError::RateLimited { .. } => {
                StatusCode::TOO_MANY_REQUESTS
            }
            AuthError::EmailInUse | AuthError::IdentityInUse | AuthError::LastIdentity => {
                StatusCode::CONFLICT
            }
            AuthError::Provider(_) => StatusCode::BAD_GATEWAY,
            AuthError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<sqlx::Error> for AuthError {
    fn from(error: sqlx::Error) -> Self {
        AuthError::Internal(error.into())
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let message = match &self {
            AuthError::Internal(error) => {
                tracing::error!(error = format!("{error:#}"), "account request failed");
                "something went wrong on the server".to_owned()
            }
            AuthError::Provider(detail) => {
                tracing::warn!(detail, "provider sign-in failed");
                self.to_string()
            }
            other => other.to_string(),
        };
        let body = Json(json!({ "error": { "code": self.code(), "message": message } }));
        let mut response = (self.status(), body).into_response();
        if let AuthError::RateLimited { retry_after } = self {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, retry_after.into());
        }
        response
    }
}
