//! # FrameQuery SDK for Rust
//!
//! Official Rust client for the [FrameQuery](https://framequery.com) video
//! processing API. Upload videos, submit URLs, poll for results, and query
//! your account quota -- all with idiomatic async Rust.
//!
//! ## Quick start
//!
//! ```no_run
//! use framequery::Client;
//!
//! #[tokio::main]
//! async fn main() -> framequery::Result<()> {
//!     let client = Client::new("fq_live_your_api_key");
//!
//!     // Upload and wait for processing to finish
//!     let result = client.process("meeting.mp4", None).await?;
//!
//!     println!("Duration: {:.1}s", result.duration);
//!     for scene in &result.scenes {
//!         println!("  [{:.1}s] {}", scene.end_time, scene.description);
//!     }
//!     for seg in &result.transcript {
//!         println!("  [{:.1}-{:.1}] {}", seg.start_time, seg.end_time, seg.text);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Builder pattern
//!
//! ```no_run
//! use framequery::ClientBuilder;
//! use std::time::Duration;
//!
//! # async fn example() -> framequery::Result<()> {
//! let client = ClientBuilder::new()
//!     .api_key("fq_live_your_api_key")
//!     .base_url("https://custom.example.com/v1/api")
//!     .max_retries(5)
//!     .timeout(Duration::from_secs(120))
//!     .build()?;
//! # Ok(())
//! # }
//! ```

mod client;
mod errors;
mod models;

pub use client::{Client, ClientBuilder};
pub use errors::{FrameQueryError, Result};
pub use models::{
    Job, JobPage, ProcessOptions, ProcessingResult, Quota, Scene, TranscriptSegment,
};
