use std::path::Path;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::de::DeserializeOwned;
use serde_json::json;
use tokio::time::Instant;

use crate::errors::{FrameQueryError, Result};
use crate::models::{
    job_from_value, processing_result_from_value, CreateJobFromUrlResponse, CreateJobResponse,
    GetJobResponse, GetQuotaResponse, Job, JobPage, ListJobsResponse, ProcessOptions,
    ProcessingResult, Quota,
};

const DEFAULT_BASE_URL: &str = "https://api.framequery.com/v1/api";
const DEFAULT_MAX_RETRIES: u32 = 3;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

/// Builder for constructing a [`Client`] with custom configuration.
///
/// # Example
///
/// ```no_run
/// use framequery::ClientBuilder;
/// use std::time::Duration;
///
/// # async fn example() -> framequery::Result<()> {
/// let client = ClientBuilder::new()
///     .api_key("fq_live_abc123")
///     .base_url("https://custom.example.com/v1/api")
///     .max_retries(5)
///     .timeout(Duration::from_secs(120))
///     .build()?;
/// # Ok(())
/// # }
/// ```
pub struct ClientBuilder {
    api_key: Option<String>,
    base_url: String,
    max_retries: u32,
    timeout: Duration,
}

impl ClientBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self {
            api_key: None,
            base_url: DEFAULT_BASE_URL.to_string(),
            max_retries: DEFAULT_MAX_RETRIES,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Set the API key for authentication.
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Override the base URL (defaults to `https://api.framequery.com/v1/api`).
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set the maximum number of retries for transient errors (defaults to 3).
    pub fn max_retries(mut self, n: u32) -> Self {
        self.max_retries = n;
        self
    }

    /// Set the HTTP request timeout (defaults to 60 seconds).
    pub fn timeout(mut self, d: Duration) -> Self {
        self.timeout = d;
        self
    }

    /// Build the [`Client`].
    ///
    /// If no API key was set via [`api_key`](Self::api_key), the builder will
    /// attempt to read the `FRAMEQUERY_API_KEY` environment variable.
    ///
    /// Returns [`FrameQueryError::Authentication`] if no key is available.
    pub fn build(self) -> Result<Client> {
        let api_key = self
            .api_key
            .or_else(|| std::env::var("FRAMEQUERY_API_KEY").ok())
            .ok_or_else(|| FrameQueryError::Authentication {
                message: "API key is required. Pass it to ClientBuilder::api_key() \
                          or set the FRAMEQUERY_API_KEY environment variable."
                    .into(),
            })?;

        let http = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(FrameQueryError::Http)?;

        Ok(Client {
            base_url: self.base_url.trim_end_matches('/').to_string(),
            api_key,
            http,
            max_retries: self.max_retries,
        })
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// The FrameQuery API client.
///
/// Use [`Client::new`] for quick construction or [`ClientBuilder`] for full control.
///
/// # Example
///
/// ```no_run
/// use framequery::Client;
///
/// # async fn example() -> framequery::Result<()> {
/// let client = Client::new("fq_live_abc123");
///
/// // Upload and process a video, blocking until complete
/// let result = client.process("video.mp4", None).await?;
/// println!("{} scenes detected", result.scenes.len());
/// # Ok(())
/// # }
/// ```
pub struct Client {
    base_url: String,
    api_key: String,
    http: reqwest::Client,
    max_retries: u32,
}

impl Client {
    /// Create a new client with the given API key and default settings.
    ///
    /// For customization, use [`ClientBuilder`] instead.
    pub fn new(api_key: impl Into<String>) -> Self {
        let api_key = api_key.into();
        let http = reqwest::Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .build()
            .expect("failed to build HTTP client");

        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            api_key,
            http,
            max_retries: DEFAULT_MAX_RETRIES,
        }
    }

    /// Upload a local video file and poll until processing completes.
    ///
    /// This is the highest-level method: it creates a job, uploads the file to
    /// the returned signed URL, then polls until the job reaches a terminal
    /// status. Use [`ProcessOptions`] to configure polling behavior and receive
    /// progress callbacks.
    ///
    /// Returns a [`ProcessingResult`] with scenes, transcript, and metadata.
    ///
    /// # Errors
    ///
    /// - [`FrameQueryError::Io`] if the file cannot be read.
    /// - [`FrameQueryError::Timeout`] if polling exceeds the configured timeout.
    /// - [`FrameQueryError::JobFailed`] if the job reaches `FAILED` status.
    pub async fn process(
        &self,
        path: impl AsRef<Path>,
        opts: Option<ProcessOptions>,
    ) -> Result<ProcessingResult> {
        let job = self.upload(path).await?;
        let opts = opts.unwrap_or_default();
        self.poll(&job.id, &opts).await
    }

    /// Submit a URL for server-side download and poll until processing completes.
    ///
    /// The FrameQuery backend will fetch the video from the provided URL directly,
    /// so there is no local upload step.
    ///
    /// # Errors
    ///
    /// - [`FrameQueryError::Timeout`] if polling exceeds the configured timeout.
    /// - [`FrameQueryError::JobFailed`] if the job reaches `FAILED` status.
    pub async fn process_url(
        &self,
        url: &str,
        opts: Option<ProcessOptions>,
    ) -> Result<ProcessingResult> {
        // Derive a filename from the URL path, or fall back to "video.mp4".
        let file_name = url
            .rsplit('/')
            .next()
            .filter(|s| !s.is_empty() && s.contains('.'))
            .unwrap_or("video.mp4");

        let body = json!({
            "url": url,
            "fileName": file_name,
        });

        let resp: CreateJobFromUrlResponse = self.request("POST", "/jobs/from-url", Some(body)).await?;
        let opts = opts.unwrap_or_default();
        self.poll(&resp.data.job_id, &opts).await
    }

    /// Upload a local video file and return immediately with the created [`Job`].
    ///
    /// This performs two HTTP calls:
    /// 1. `POST /jobs` to create the job and obtain a signed upload URL.
    /// 2. `PUT` the file binary to the signed URL.
    ///
    /// The returned [`Job`] will typically be in a non-terminal status. Use
    /// [`get_job`](Self::get_job) to check progress, or [`process`](Self::process)
    /// for a fire-and-forget workflow.
    pub async fn upload(&self, path: impl AsRef<Path>) -> Result<Job> {
        let path = path.as_ref();

        // Validate the file exists and read it into memory.
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "video.mp4".to_string());

        let file_bytes = tokio::fs::read(path).await.map_err(FrameQueryError::Io)?;

        // Step 1: Create the job.
        let body = json!({ "fileName": file_name });
        let resp: CreateJobResponse = self.request("POST", "/jobs", Some(body)).await?;

        // Step 2: Upload file to signed URL.
        let upload_resp = self
            .http
            .put(&resp.data.upload_url)
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(file_bytes)
            .send()
            .await
            .map_err(FrameQueryError::Http)?;

        if !upload_resp.status().is_success() {
            let status = upload_resp.status().as_u16();
            let text = upload_resp.text().await.unwrap_or_default();
            return Err(FrameQueryError::Api {
                status_code: status,
                message: format!("upload to signed URL failed: {text}"),
                body: None,
            });
        }

        // Return a Job struct representing the freshly created job.
        Ok(Job {
            id: resp.data.job_id.clone(),
            status: "PENDING_ORCHESTRATION".to_string(),
            filename: file_name,
            created_at: String::new(),
            eta_seconds: None,
            raw: json!({
                "jobId": resp.data.job_id,
                "status": "PENDING_ORCHESTRATION",
            }),
        })
    }

    /// Fetch the current state of a job by its identifier.
    pub async fn get_job(&self, job_id: &str) -> Result<Job> {
        let resp: GetJobResponse = self
            .request("GET", &format!("/jobs/{job_id}"), None)
            .await?;
        Ok(job_from_value(resp.data))
    }

    /// List jobs with optional filtering and pagination.
    ///
    /// # Parameters
    ///
    /// - `limit` — Maximum number of jobs to return per page (default decided by the API).
    /// - `cursor` — Cursor from a previous [`JobPage::next_cursor`] for pagination.
    /// - `status` — Filter to only jobs with this status (e.g. `"COMPLETED"`).
    pub async fn list_jobs(
        &self,
        limit: Option<u32>,
        cursor: Option<&str>,
        status: Option<&str>,
    ) -> Result<JobPage> {
        let mut query_parts: Vec<String> = Vec::new();

        if let Some(l) = limit {
            query_parts.push(format!("limit={l}"));
        }
        if let Some(c) = cursor {
            query_parts.push(format!("cursor={c}"));
        }
        if let Some(s) = status {
            query_parts.push(format!("status={s}"));
        }

        let path = if query_parts.is_empty() {
            "/jobs".to_string()
        } else {
            format!("/jobs?{}", query_parts.join("&"))
        };

        let resp: ListJobsResponse = self.request("GET", &path, None).await?;

        let jobs = resp.data.into_iter().map(job_from_value).collect();

        Ok(JobPage {
            jobs,
            next_cursor: resp.next_cursor,
        })
    }

    /// Retrieve the authenticated user's current quota and billing information.
    pub async fn get_quota(&self) -> Result<Quota> {
        let resp: GetQuotaResponse = self.request("GET", "/quota", None).await?;
        Ok(resp.data)
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Execute an HTTP request with automatic retry for transient failures.
    ///
    /// Retries are performed for:
    /// - HTTP 5xx server errors
    /// - HTTP 429 rate-limit responses
    /// - Network-level errors (connection refused, timeout, etc.)
    ///
    /// Exponential backoff is applied: 1s, 2s, 4s, ...
    async fn request<T: DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.api_key))
                .expect("invalid API key characters"),
        );

        let mut last_err: Option<FrameQueryError> = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let backoff = Duration::from_secs(1 << (attempt - 1).min(5));
                tokio::time::sleep(backoff).await;
            }

            let mut req = match method {
                "GET" => self.http.get(&url),
                "POST" => self.http.post(&url),
                "PUT" => self.http.put(&url),
                "DELETE" => self.http.delete(&url),
                "PATCH" => self.http.patch(&url),
                _ => self.http.get(&url),
            };

            req = req.headers(headers.clone());

            if let Some(ref b) = body {
                req = req.header(CONTENT_TYPE, "application/json").json(b);
            }

            let response = match req.send().await {
                Ok(r) => r,
                Err(e) => {
                    // Network-level error: retry if we have attempts left.
                    last_err = Some(FrameQueryError::Http(e));
                    continue;
                }
            };

            let status = response.status();

            // Successful response: deserialize and return.
            if status.is_success() {
                let value: T = response.json().await.map_err(FrameQueryError::Http)?;
                return Ok(value);
            }

            // Map well-known error codes to typed errors.
            let status_code = status.as_u16();
            let response_text = response.text().await.unwrap_or_default();

            let parsed_body: Option<serde_json::Value> =
                serde_json::from_str(&response_text).ok();

            let message = parsed_body
                .as_ref()
                .and_then(|b| b.get("error"))
                .and_then(|e| e.as_str())
                .unwrap_or(&response_text)
                .to_string();

            let err = match status_code {
                401 => FrameQueryError::Authentication { message },
                403 => FrameQueryError::PermissionDenied { message },
                404 => FrameQueryError::NotFound { message },
                429 => {
                    // Extract Retry-After header if present.
                    let retry_after = parsed_body
                        .as_ref()
                        .and_then(|b| b.get("retryAfter"))
                        .and_then(|v| v.as_f64());

                    FrameQueryError::RateLimit {
                        message,
                        retry_after,
                    }
                }
                _ => FrameQueryError::Api {
                    status_code,
                    message,
                    body: parsed_body,
                },
            };

            // Retry on 5xx or 429; return immediately for other errors.
            if status_code >= 500 || status_code == 429 {
                last_err = Some(err);
                continue;
            }

            return Err(err);
        }

        // All retries exhausted.
        Err(last_err.unwrap_or_else(|| FrameQueryError::Api {
            status_code: 0,
            message: "request failed after all retries".into(),
            body: None,
        }))
    }

    /// Poll a job until it reaches a terminal status or the timeout is exceeded.
    async fn poll(&self, job_id: &str, opts: &ProcessOptions) -> Result<ProcessingResult> {
        let deadline = Instant::now() + opts.timeout;

        loop {
            let job = self.get_job(job_id).await?;

            if let Some(ref cb) = opts.on_progress {
                cb(&job);
            }

            if job.is_failed() {
                return Err(FrameQueryError::JobFailed(format!(
                    "job {} reached FAILED status",
                    job.id
                )));
            }

            if job.is_complete() {
                return Ok(processing_result_from_value(job.raw));
            }

            if Instant::now() >= deadline {
                return Err(FrameQueryError::Timeout(opts.timeout));
            }

            tokio::time::sleep(opts.poll_interval).await;
        }
    }
}
