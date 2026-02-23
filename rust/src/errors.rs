use thiserror::Error;

#[derive(Error, Debug)]
pub enum FrameQueryError {
    /// HTTP 401.
    #[error("authentication failed: {message}")]
    Authentication { message: String },

    /// HTTP 403.
    #[error("permission denied: {message}")]
    PermissionDenied { message: String },

    /// HTTP 404.
    #[error("not found: {message}")]
    NotFound { message: String },

    /// HTTP 429. `retry_after` comes from the response body, if present.
    #[error("rate limited (retry after {retry_after:?}s): {message}")]
    RateLimit {
        message: String,
        retry_after: Option<f64>,
    },

    /// Any other non-2xx response. `body` has the parsed JSON if it was valid.
    #[error("API error {status_code}: {message}")]
    Api {
        status_code: u16,
        message: String,
        body: Option<serde_json::Value>,
    },

    /// reqwest transport error (DNS, TLS, connection reset, etc.).
    #[error("request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// File read failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Polling exceeded the configured timeout.
    #[error("poll timed out after {0:?}")]
    Timeout(std::time::Duration),

    /// Job status became `FAILED`.
    #[error("job failed: {0}")]
    JobFailed(String),
}

pub type Result<T> = std::result::Result<T, FrameQueryError>;
