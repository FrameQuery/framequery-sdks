use thiserror::Error;

/// All errors that can occur when using the FrameQuery SDK.
#[derive(Error, Debug)]
pub enum FrameQueryError {
    /// The API key is missing or invalid (HTTP 401).
    #[error("authentication failed: {message}")]
    Authentication { message: String },

    /// The authenticated user does not have access to the requested resource (HTTP 403).
    #[error("permission denied: {message}")]
    PermissionDenied { message: String },

    /// The requested resource was not found (HTTP 404).
    #[error("not found: {message}")]
    NotFound { message: String },

    /// The request was rate-limited (HTTP 429).
    #[error("rate limited (retry after {retry_after:?}s): {message}")]
    RateLimit {
        message: String,
        retry_after: Option<f64>,
    },

    /// A non-specific API error with the HTTP status code and response body.
    #[error("API error {status_code}: {message}")]
    Api {
        status_code: u16,
        message: String,
        body: Option<serde_json::Value>,
    },

    /// A transport-level HTTP error from reqwest.
    #[error("request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// An I/O error, typically from reading a local file.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Polling for job completion exceeded the configured timeout.
    #[error("poll timed out after {0:?}")]
    Timeout(std::time::Duration),

    /// The job reached a terminal FAILED status.
    #[error("job failed: {0}")]
    JobFailed(String),
}

/// A convenience alias for `Result<T, FrameQueryError>`.
pub type Result<T> = std::result::Result<T, FrameQueryError>;
