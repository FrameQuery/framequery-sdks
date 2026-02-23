# framequery

Rust client for the [FrameQuery](https://framequery.com) API.

## Install

```toml
[dependencies]
framequery = "0.1"
tokio = { version = "1", features = ["full"] }
```

## Usage

```rust
use framequery::Client;

#[tokio::main]
async fn main() -> framequery::Result<()> {
    let client = Client::new("fq_live_your_api_key");

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

## ClientBuilder

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

Falls back to `FRAMEQUERY_API_KEY` env var if `.api_key()` is not called.

## Process a URL

```rust
let result = client
    .process_url("https://example.com/video.mp4", None)
    .await?;
```

## Upload without waiting

```rust
let job = client.upload("video.mp4").await?;
println!("Job ID: {}", job.id);

// later
let job = client.get_job(&job.id).await?;
if job.is_complete() {
    println!("Done!");
}
```

## Progress callbacks

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

## Pagination

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

## Quota

```rust
let quota = client.get_quota().await?;
println!("Plan: {}", quota.plan);
println!("Included hours: {:.1}", quota.included_hours);
println!("Credits: {:.1}h", quota.credits_balance_hours);
```

## Error handling

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

## Retries

5xx, 429, and network errors are retried with exponential backoff (1s, 2s, 4s, ...). Default: 3 retries. Configurable via `ClientBuilder::max_retries`.

## API

| Method | Returns |
|---|---|
| `client.process(path, opts)` | Upload + poll to completion |
| `client.process_url(url, opts)` | Submit URL + poll to completion |
| `client.upload(path)` | Upload, return `Job` immediately |
| `client.get_job(id)` | Current job state |
| `client.list_jobs(limit, cursor, status)` | Paginated job list |
| `client.get_quota()` | Quota and billing info |

Every `Job` and `ProcessingResult` has a `.raw` field with the full JSON response.

## License

MIT
