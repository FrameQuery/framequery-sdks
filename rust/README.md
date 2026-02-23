# FrameQuery Rust SDK

Official Rust client for the [FrameQuery](https://framequery.com) video processing API. Upload videos, submit URLs, poll for results, and query your account quota with idiomatic async Rust.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
framequery = "0.1"
tokio = { version = "1", features = ["full"] }
```

## Quick start

```rust
use framequery::Client;

#[tokio::main]
async fn main() -> framequery::Result<()> {
    let client = Client::new("fq_live_your_api_key");

    // Upload a video and wait for processing to complete
    let result = client.process("meeting.mp4", None).await?;

    println!("Duration: {:.1}s", result.duration);
    for scene in &result.scenes {
        println!("[{:.1}s] {}", scene.end_time, scene.description);
    }
    for seg in &result.transcript {
        println!("[{:.1}-{:.1}] {}", seg.start_time, seg.end_time, seg.text);
    }

    Ok(())
}
```

## Builder pattern

Use `ClientBuilder` for full control over the client configuration:

```rust
use framequery::ClientBuilder;
use std::time::Duration;

let client = ClientBuilder::new()
    .api_key("fq_live_your_api_key")
    .base_url("https://custom.example.com/v1/api")
    .max_retries(5)
    .timeout(Duration::from_secs(120))
    .build()?;
```

If you omit `.api_key()`, the builder reads the `FRAMEQUERY_API_KEY` environment variable automatically.

## Process a remote URL

Submit a publicly accessible URL for server-side download and processing:

```rust
let result = client
    .process_url("https://example.com/video.mp4", None)
    .await?;
```

## Upload without waiting

If you want to upload a file and check the result later:

```rust
let job = client.upload("video.mp4").await?;
println!("Job ID: {}", job.id);

// Later...
let job = client.get_job(&job.id).await?;
if job.is_complete() {
    println!("Done!");
}
```

## Progress callbacks

Monitor polling progress with a callback:

```rust
use framequery::ProcessOptions;
use std::time::Duration;

let opts = ProcessOptions {
    poll_interval: Duration::from_secs(3),
    timeout: Duration::from_secs(600),
    on_progress: Some(Box::new(|job| {
        println!("Status: {}", job.status);
        if let Some(eta) = job.eta_seconds {
            println!("  ETA: {:.0}s", eta);
        }
    })),
};

let result = client.process("video.mp4", Some(opts)).await?;
```

## List jobs with pagination

```rust
let mut cursor: Option<String> = None;
loop {
    let page = client
        .list_jobs(Some(20), cursor.as_deref(), Some("COMPLETED"))
        .await?;

    for job in &page.jobs {
        println!("{} | {} | {}", job.id, job.status, job.filename);
    }

    if !page.has_more() {
        break;
    }
    cursor = page.next_cursor;
}
```

## Check quota

```rust
let quota = client.get_quota().await?;
println!("Plan: {}", quota.plan);
println!("Included hours: {:.1}", quota.included_hours);
println!("Credits: {:.1}h", quota.credits_balance_hours);
```

## Error handling

All methods return `framequery::Result<T>`. Match on `FrameQueryError` variants for fine-grained control:

```rust
use framequery::FrameQueryError;

match client.get_job("nonexistent").await {
    Ok(job) => println!("Found: {}", job.status),
    Err(FrameQueryError::NotFound { message }) => {
        eprintln!("Job not found: {}", message);
    }
    Err(FrameQueryError::Authentication { message }) => {
        eprintln!("Auth failed: {}", message);
    }
    Err(FrameQueryError::RateLimit { retry_after, .. }) => {
        eprintln!("Rate limited, retry after {:?}s", retry_after);
    }
    Err(FrameQueryError::Timeout(duration)) => {
        eprintln!("Timed out after {:?}", duration);
    }
    Err(FrameQueryError::JobFailed(msg)) => {
        eprintln!("Processing failed: {}", msg);
    }
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Features

- **Async/await** -- built on `tokio` and `reqwest` for efficient non-blocking I/O.
- **Automatic retries** -- transient errors (5xx, 429, network failures) are retried with exponential backoff.
- **Builder pattern** -- configure base URL, timeouts, retry count, and API key source.
- **Typed errors** -- `FrameQueryError` enum maps HTTP status codes to descriptive variants.
- **Progress callbacks** -- optional closure called on each poll iteration.
- **Pagination** -- cursor-based iteration over job lists.
- **Raw access** -- every `Job` and `ProcessingResult` includes the full JSON response in `.raw`.

## API reference

| Method | Description |
|---|---|
| `client.process(path, opts)` | Upload + poll until complete |
| `client.process_url(url, opts)` | Submit URL + poll until complete |
| `client.upload(path)` | Upload only, return immediately |
| `client.get_job(id)` | Fetch current job state |
| `client.list_jobs(limit, cursor, status)` | List jobs with pagination |
| `client.get_quota()` | Get account quota and billing info |

## License

MIT
