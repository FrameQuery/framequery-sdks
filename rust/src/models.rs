use serde::{Deserialize, Serialize};

/// A single scene detected in the video.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Scene {
    /// Human-readable description of the scene contents.
    pub description: String,

    /// End timestamp of the scene in seconds from the start of the video.
    #[serde(rename = "endTs")]
    pub end_time: f64,

    /// Objects detected within this scene (e.g. "person", "car").
    #[serde(default)]
    pub objects: Vec<String>,
}

/// A single segment of the audio transcript.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TranscriptSegment {
    /// Start time in seconds.
    #[serde(rename = "StartTime")]
    pub start_time: f64,

    /// End time in seconds.
    #[serde(rename = "EndTime")]
    pub end_time: f64,

    /// Transcribed text for this segment.
    #[serde(rename = "Text")]
    pub text: String,
}

/// The fully processed result of a completed video job.
///
/// Returned by [`Client::process`] and [`Client::process_url`] after the job
/// reaches a terminal `COMPLETED` status.
#[derive(Debug, Clone)]
pub struct ProcessingResult {
    /// The unique job identifier.
    pub job_id: String,

    /// Terminal status string (e.g. "COMPLETED", "COMPLETED_NO_SCENES").
    pub status: String,

    /// Original filename of the uploaded video.
    pub filename: String,

    /// Duration of the video in seconds.
    pub duration: f64,

    /// Scene descriptions extracted from the video frames.
    pub scenes: Vec<Scene>,

    /// Audio transcript segments.
    pub transcript: Vec<TranscriptSegment>,

    /// ISO 8601 timestamp when the job was created.
    pub created_at: String,

    /// The full raw JSON response from the API for advanced usage.
    pub raw: serde_json::Value,
}

/// A snapshot of a processing job's current state.
///
/// Returned by [`Client::upload`], [`Client::get_job`], and [`Client::list_jobs`].
#[derive(Debug, Clone)]
pub struct Job {
    /// The unique job identifier.
    pub id: String,

    /// Current status string (e.g. "PENDING_ORCHESTRATION", "COMPLETED", "FAILED").
    pub status: String,

    /// Original filename of the uploaded video.
    pub filename: String,

    /// ISO 8601 timestamp when the job was created.
    pub created_at: String,

    /// Estimated seconds remaining until completion, if available.
    pub eta_seconds: Option<f64>,

    /// The full raw JSON response from the API for advanced usage.
    pub raw: serde_json::Value,
}

impl Job {
    /// Returns `true` if the job has reached a terminal status and will not change further.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status.as_str(),
            "COMPLETED" | "COMPLETED_NO_SCENES" | "FAILED"
        )
    }

    /// Returns `true` if the job completed successfully (with or without scenes).
    pub fn is_complete(&self) -> bool {
        matches!(self.status.as_str(), "COMPLETED" | "COMPLETED_NO_SCENES")
    }

    /// Returns `true` if the job has failed.
    pub fn is_failed(&self) -> bool {
        self.status == "FAILED"
    }
}

/// Quota information for the authenticated user.
#[derive(Debug, Clone, Deserialize)]
pub struct Quota {
    /// The user's current subscription plan (e.g. "free", "starter", "pro", "enterprise").
    pub plan: String,

    /// Included hours that reset each billing period.
    #[serde(rename = "includedHours")]
    pub included_hours: f64,

    /// Purchased credit hours balance that never expires.
    #[serde(rename = "creditsBalanceHours")]
    pub credits_balance_hours: f64,

    /// ISO 8601 timestamp when included hours reset, if applicable.
    #[serde(rename = "resetDate")]
    pub reset_date: Option<String>,
}

/// A paginated page of job summaries.
#[derive(Debug, Clone)]
pub struct JobPage {
    /// The jobs on this page.
    pub jobs: Vec<Job>,

    /// Cursor to pass as the `cursor` parameter to fetch the next page.
    /// `None` when there are no more results.
    pub next_cursor: Option<String>,
}

impl JobPage {
    /// Returns `true` if there is another page of results available.
    pub fn has_more(&self) -> bool {
        self.next_cursor.is_some()
    }
}

/// Options for the high-level [`Client::process`] and [`Client::process_url`] methods.
pub struct ProcessOptions {
    /// How often to poll the job status. Defaults to 5 seconds.
    pub poll_interval: std::time::Duration,

    /// Maximum time to wait for the job to complete. Defaults to 24 hours.
    pub timeout: std::time::Duration,

    /// Optional callback invoked on each poll with the current job state.
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

/// Envelope returned by POST /jobs.
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

/// Envelope returned by POST /jobs/from-url.
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

/// Envelope returned by GET /jobs/{jobId}.
#[derive(Deserialize)]
pub(crate) struct GetJobResponse {
    pub data: serde_json::Value,
}

/// Envelope returned by GET /jobs (list).
#[derive(Deserialize)]
pub(crate) struct ListJobsResponse {
    pub data: Vec<serde_json::Value>,
    #[serde(rename = "nextCursor")]
    pub next_cursor: Option<String>,
}

/// Envelope returned by GET /quota.
#[derive(Deserialize)]
pub(crate) struct GetQuotaResponse {
    pub data: Quota,
}

/// Helper: extract an `&str` from a `serde_json::Value` by key, returning `""` if missing.
pub(crate) fn json_str(val: &serde_json::Value, key: &str) -> String {
    val.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// Helper: extract an `Option<f64>` from a JSON value by key.
pub(crate) fn json_f64_opt(val: &serde_json::Value, key: &str) -> Option<f64> {
    val.get(key).and_then(|v| v.as_f64())
}

/// Convert a raw job JSON value into a [`Job`] struct.
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

/// Convert a raw completed-job JSON value into a [`ProcessingResult`].
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
