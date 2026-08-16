//! Errors surfaced by the sigit.si API client.

/// The API's error body — `Api::BaseController#render_error` renders every
/// failure as `{ "error_code": <i32>, "message": <string> }`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ErrorResponse {
    pub error_code: i32,
    pub message: String,
}

/// `error_codes::ErrorCode` values the server sends, mirrored from
/// `Api::BaseController`.
pub mod codes {
    pub const UNKNOWN: i32 = 0;
    pub const UNAUTHORIZED: i32 = 100;
    pub const INVALID: i32 = 101;
}

pub type Result<T> = std::result::Result<T, ApiError>;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// The token is missing, expired, or rejected. Callers treat this as
    /// "clear the stored token and ask the user to sign in again", so it stays
    /// a distinct variant rather than folding into [`ApiError::Status`].
    #[error("Your session has expired. Sign in again with `si auth login`.")]
    Unauthorized,

    /// No token on disk for this environment.
    #[error("You are not signed in. Run `si auth login` first.")]
    NotAuthenticated,

    /// The server answered with a non-2xx status and an `ErrorResponse` body.
    #[error("{message}")]
    Status {
        status: u16,
        error_code: i32,
        message: String,
    },

    /// An MCP tool reported an in-band failure (`isError: true`). Issues and
    /// pull requests go over MCP, so this is a normal user-facing error.
    #[error("{0}")]
    Tool(String),

    /// The request never completed — DNS, TLS, connection refused, timeout.
    #[error("Could not reach {host}: {source}")]
    Transport {
        host: String,
        #[source]
        source: reqwest::Error,
    },

    /// A 2xx body that didn't match the expected shape.
    #[error("Unexpected response from the server: {0}")]
    Parse(String),
}

impl ApiError {
    /// True when the caller should discard the stored credentials.
    pub fn is_auth_failure(&self) -> bool {
        matches!(self, ApiError::Unauthorized | ApiError::NotAuthenticated)
            || matches!(self, ApiError::Status { status: 401, .. })
    }
}
