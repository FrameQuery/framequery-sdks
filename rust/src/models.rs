use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Scene {
    pub description: String,

    /// Seconds from video start.
    #[serde(rename = "endTs")]
    pub end_time: f64,

    /// e.g. "person", "car".
    #[serde(default)]
    pub objects: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TranscriptSegment {
    #[serde(rename = "StartTime")]
    pub start_time: f64,

    #[serde(rename = "EndTime")]
    pub end_time: f64,

    #[serde(rename = "Text")]
    pub text: String,
}

/// Returned by `process` / `process_url` once the job completes.
#[derive(Debug, Clone)]
pub struct ProcessingResult {
    pub job_id: String,
    /// "COMPLETED" or "COMPLETED_NO_SCENES".
    pub status: String,
    pub filename: String,
    /// Video length in seconds.
    pub duration: f64,
    pub scenes: Vec<Scene>,
    pub transcript: Vec<TranscriptSegment>,
    /// ISO 8601.
    pub created_at: String,
    /// Full API response JSON.
    pub raw: serde_json::Value,
}

/// Current state of a job. Check `status` or use the `is_*` helpers.
#[derive(Debug, Clone)]
pub struct Job {
    pub id: String,
    pub status: String,
    pub filename: String,
    /// ISO 8601.
    pub created_at: String,
    /// Server-provided ETA, if any.
    pub eta_seconds: Option<f64>,
    /// Full API response JSON.
    pub raw: serde_json::Value,
}

impl Job {
    /// Terminal = won't change anymore (COMPLETED, COMPLETED_NO_SCENES, or FAILED).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status.as_str(),
            "COMPLETED" | "COMPLETED_NO_SCENES" | "FAILED"
        )
    }

    /// COMPLETED or COMPLETED_NO_SCENES.
    pub fn is_complete(&self) -> bool {
        matches!(self.status.as_str(), "COMPLETED" | "COMPLETED_NO_SCENES")
    }

    /// Status is FAILED.
    pub fn is_failed(&self) -> bool {
        self.status == "FAILED"
    }

    /// Parse `processedData` from the raw response into a [`ProcessingResult`].
    /// Returns `None` if the job isn't complete or has no processed data.
    pub fn result(&self) -> Option<ProcessingResult> {
        if !self.is_complete() {
            return None;
        }
        self.raw.get("processedData")?;
        Some(processing_result_from_value(self.raw.clone()))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Quota {
    /// "free", "starter", "pro", or "enterprise".
    pub plan: String,

    /// Resets each billing period.
    #[serde(rename = "includedHours")]
    pub included_hours: f64,

    /// Purchased credits, never expire.
    #[serde(rename = "creditsBalanceHours")]
    pub credits_balance_hours: f64,

    /// When included hours reset. ISO 8601.
    #[serde(rename = "resetDate")]
    pub reset_date: Option<String>,
}

#[derive(Debug, Clone)]
pub struct JobPage {
    pub jobs: Vec<Job>,
    /// Pass to `list_jobs` for the next page. `None` means no more results.
    pub next_cursor: Option<String>,
}

impl JobPage {
    /// `true` if `next_cursor` is `Some`.
    pub fn has_more(&self) -> bool {
        self.next_cursor.is_some()
    }
}

/// Polling config for `process` / `process_url`.
pub struct ProcessOptions {
    /// Default: 5s.
    pub poll_interval: std::time::Duration,
    /// Default: 24h.
    pub timeout: std::time::Duration,
    /// Called on each poll iteration with the current `Job`.
    #[allow(clippy::type_complexity)]
    pub on_progress: Option<Box<dyn Fn(&Job) + Send>>,
}

impl Default for ProcessOptions {
    fn default() -> Self {
        Self {
            poll_interval: std::time::Duration::from_secs(5),
            timeout: std::time::Duration::from_secs(24 * 60 * 60),
            on_progress: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Internal deserialization helpers (not part of the public API surface)
// ---------------------------------------------------------------------------

/// POST /jobs response.
#[derive(Deserialize)]
pub(crate) struct CreateJobResponse {
    pub data: CreateJobData,
}

#[derive(Deserialize)]
pub(crate) struct CreateJobData {
    #[serde(rename = "jobId")]
    pub job_id: String,
    #[serde(rename = "uploadUrl")]
    pub upload_url: String,
    #[serde(default, rename = "expiresInSeconds")]
    #[allow(dead_code)]
    pub expires_in_seconds: Option<u64>,
    #[serde(default, rename = "uploadMethod")]
    #[allow(dead_code)]
    pub upload_method: Option<String>,
}

/// POST /jobs/from-url response.
#[derive(Deserialize)]
pub(crate) struct CreateJobFromUrlResponse {
    pub data: CreateJobFromUrlData,
}

#[derive(Deserialize)]
pub(crate) struct CreateJobFromUrlData {
    #[serde(rename = "jobId")]
    pub job_id: String,
    #[allow(dead_code)]
    pub status: String,
}

/// GET /jobs/{jobId} response.
#[derive(Deserialize)]
pub(crate) struct GetJobResponse {
    pub data: serde_json::Value,
}

/// GET /jobs response.
#[derive(Deserialize)]
pub(crate) struct ListJobsResponse {
    pub data: Vec<serde_json::Value>,
    #[serde(rename = "nextCursor")]
    pub next_cursor: Option<String>,
}

/// GET /quota response.
#[derive(Deserialize)]
pub(crate) struct GetQuotaResponse {
    pub data: Quota,
}

/// Pull a string out of a JSON value, or `""` if missing.
pub(crate) fn json_str(val: &serde_json::Value, key: &str) -> String {
    val.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// Pull an `f64` out of a JSON value, or `None`.
pub(crate) fn json_f64_opt(val: &serde_json::Value, key: &str) -> Option<f64> {
    val.get(key).and_then(|v| v.as_f64())
}

/// Parse a raw job JSON value into a [`Job`].
pub(crate) fn job_from_value(val: serde_json::Value) -> Job {
    Job {
        id: json_str(&val, "jobId"),
        status: json_str(&val, "status"),
        filename: json_str(&val, "originalFilename"),
        created_at: json_str(&val, "createdAt"),
        eta_seconds: json_f64_opt(&val, "estimatedCompletionTimeSeconds"),
        raw: val,
    }
}

/// Parse a completed job's JSON into a [`ProcessingResult`].
pub(crate) fn processing_result_from_value(val: serde_json::Value) -> ProcessingResult {
    let processed = val.get("processedData").cloned().unwrap_or_default();

    let duration = processed
        .get("length")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let scenes: Vec<Scene> = processed
        .get("scenes")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let transcript: Vec<TranscriptSegment> = processed
        .get("transcript")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    ProcessingResult {
        job_id: json_str(&val, "jobId"),
        status: json_str(&val, "status"),
        filename: json_str(&val, "originalFilename"),
        duration,
        scenes,
        transcript,
        created_at: json_str(&val, "createdAt"),
        raw: val,
    }
}
