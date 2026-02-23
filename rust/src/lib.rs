//! Rust client for the [FrameQuery](https://framequery.com) API.
//!
//! ```no_run
//! use framequery::Client;
//!
//! #[tokio::main]
//! async fn main() -> framequery::Result<()> {
//!     let client = Client::new("fq_live_your_api_key");
//!     let result = client.process("meeting.mp4", None).await?;
//!
//!     for scene in &result.scenes {
//!         println!("[{:.1}s] {}", scene.end_time, scene.description);
//!     }
//!     Ok(())
//! }
//! ```
//!
//! Use [`ClientBuilder`] to configure base URL, timeouts, and retry count.
//! Falls back to `FRAMEQUERY_API_KEY` env var if no key is passed explicitly.

mod client;
mod errors;
mod models;

pub use client::{Client, ClientBuilder};
pub use errors::{FrameQueryError, Result};
pub use models::{
    Job, JobPage, ProcessOptions, ProcessingResult, Quota, Scene, TranscriptSegment,
};
