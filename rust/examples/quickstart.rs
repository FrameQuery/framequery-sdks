//! Quick-start examples for the FrameQuery Rust SDK.
//!
//! Run with:
//!   FRAMEQUERY_API_KEY=fq_live_... cargo run --example quickstart
//!
//! Or pass the key directly in code (not recommended for production).

use framequery::{Client, ClientBuilder, ProcessOptions};
use std::time::Duration;

#[tokio::main]
async fn main() -> framequery::Result<()> {
    // -----------------------------------------------------------------------
    // 1. Create a client (reads FRAMEQUERY_API_KEY from environment)
    // -----------------------------------------------------------------------
    let client = ClientBuilder::new().build()?;

    // Or provide the key directly:
    // let client = Client::new("fq_live_abc123");

    // -----------------------------------------------------------------------
    // 2. Check your quota
    // -----------------------------------------------------------------------
    let quota = client.get_quota().await?;
    println!("Plan: {}", quota.plan);
    println!("Included hours: {:.1}", quota.included_hours);
    println!("Credits balance: {:.1}h", quota.credits_balance_hours);
    if let Some(ref date) = quota.reset_date {
        println!("Resets: {}", date);
    }
    println!();

    // -----------------------------------------------------------------------
    // 3. Process a local file (upload + poll until complete)
    // -----------------------------------------------------------------------
    let result = client.process("demo.mp4", None).await?;

    println!("Job {} completed!", result.job_id);
    println!("Duration: {:.1}s", result.duration);
    println!("Scenes:");
    for scene in &result.scenes {
        println!(
            "  [{:.1}s] {} (objects: {})",
            scene.end_time,
            scene.description,
            scene.objects.join(", ")
        );
    }
    println!("Transcript:");
    for seg in &result.transcript {
        println!(
            "  [{:.1}s - {:.1}s] {}",
            seg.start_time, seg.end_time, seg.text
        );
    }
    println!();

    // -----------------------------------------------------------------------
    // 4. Process a remote URL with progress callback
    // -----------------------------------------------------------------------
    let opts = ProcessOptions {
        poll_interval: Duration::from_secs(3),
        timeout: Duration::from_secs(600),
        on_progress: Some(Box::new(|job| {
            print!("  Status: {}", job.status);
            if let Some(eta) = job.eta_seconds {
                print!(" (ETA: {:.0}s)", eta);
            }
            println!();
        })),
    };

    let result = client
        .process_url("https://example.com/sample.mp4", Some(opts))
        .await?;

    println!("URL job {} completed with {} scenes.", result.job_id, result.scenes.len());
    println!();

    // -----------------------------------------------------------------------
    // 5. Upload without waiting (fire-and-forget)
    // -----------------------------------------------------------------------
    let job = client.upload("another_video.mp4").await?;
    println!("Uploaded! Job ID: {} (status: {})", job.id, job.status);

    // Check it later:
    let job = client.get_job(&job.id).await?;
    println!("Current status: {}", job.status);
    if job.is_terminal() {
        println!("Job is done (complete={}, failed={})", job.is_complete(), job.is_failed());
    }
    println!();

    // -----------------------------------------------------------------------
    // 6. List jobs with pagination
    // -----------------------------------------------------------------------
    let mut cursor: Option<String> = None;
    loop {
        let page = client
            .list_jobs(Some(10), cursor.as_deref(), Some("COMPLETED"))
            .await?;

        for job in &page.jobs {
            println!("  {} | {} | {}", job.id, job.status, job.filename);
        }

        if !page.has_more() {
            break;
        }
        cursor = page.next_cursor;
    }

    Ok(())
}
