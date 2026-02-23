# FrameQuery SDKs

Official client libraries for the [FrameQuery](https://framequery.com) video processing API.

Upload videos, process them with AI-powered scene detection and transcription, and retrieve structured results — all behind a single function call.

## Languages

| Language | Package | Install |
|---|---|---|
| [Python](./python) | `framequery` | `pip install framequery` |
| [TypeScript / Node.js](./typescript) | `framequery` | `npm install framequery` |
| [Go](./go) | `framequery-go` | `go get github.com/framequery/framequery-go` |
| [Rust](./rust) | `framequery` | `cargo add framequery` |
| [Ruby](./ruby) | `framequery` | `gem install framequery` |

## Quick Start

Every SDK follows the same high-level pattern:

1. Create a client with your API key
2. Call `process()` with a file path or URL
3. Get back typed scenes + transcript

### Python

```python
from framequery import FrameQuery

fq = FrameQuery(api_key="fq_...")

result = fq.process("interview.mp4")
print(f"Duration: {result.duration}s")
for scene in result.scenes:
    print(f"  [{scene.end_time}s] {scene.description}")
for seg in result.transcript:
    print(f"  [{seg.start_time}-{seg.end_time}s] {seg.text}")
```

### TypeScript

```typescript
import FrameQuery from "framequery";

const fq = new FrameQuery({ apiKey: "fq_..." });

const result = await fq.process("./interview.mp4");
console.log(`Duration: ${result.duration}s`);
result.scenes.forEach((s) => console.log(`  [${s.endTime}s] ${s.description}`));
result.transcript.forEach((t) => console.log(`  [${t.startTime}-${t.endTime}s] ${t.text}`));
```

### Go

```go
client := framequery.New("fq_...")

result, err := client.Process(ctx, "interview.mp4", nil)
if err != nil {
    log.Fatal(err)
}
fmt.Printf("Duration: %.1fs\n", result.Duration)
for _, s := range result.Scenes {
    fmt.Printf("  [%.1fs] %s\n", s.EndTime, s.Description)
}
```

### Rust

```rust
let client = framequery::Client::new("fq_...");

let result = client.process("interview.mp4", None).await?;
println!("Duration: {}s", result.duration);
for scene in &result.scenes {
    println!("  [{}s] {}", scene.end_time, scene.description);
}
```

### Ruby

```ruby
client = FrameQuery::Client.new(api_key: "fq_...")

result = client.process("interview.mp4")
puts "Duration: #{result.duration}s"
result.scenes.each { |s| puts "  [#{s.end_time}s] #{s.description}" }
```

## API Key

Get your API key from the [FrameQuery dashboard](https://app.framequery.com/settings/api-keys).

All SDKs read `FRAMEQUERY_API_KEY` from the environment if no key is passed explicitly:

```bash
export FRAMEQUERY_API_KEY=fq_...
```

## Common Patterns

### Process from URL (no local file needed)

```python
result = fq.process_url("https://cdn.example.com/video.mp4")
```

### Upload without waiting

```python
job = fq.upload("video.mp4")
print(job.id)  # available immediately

# Later...
job = fq.get_job(job.id)
if job.is_terminal:
    print(job.status)
```

### Progress tracking

```python
def on_progress(job):
    print(f"Status: {job.status}, ETA: {job.eta_seconds}s")

result = fq.process("video.mp4", on_progress=on_progress)
```

### Check quota

```python
quota = fq.get_quota()
print(f"{quota.plan}: {quota.credits_balance_hours}h credits remaining")
```

## API Reference

All SDKs wrap the same REST endpoints:

| Method | Endpoint | Description |
|---|---|---|
| `POST` | `/v1/api/jobs` | Create job + get upload URL |
| `POST` | `/v1/api/jobs/from-url` | Create job from remote URL |
| `GET` | `/v1/api/jobs/{jobId}` | Get job status + results |
| `GET` | `/v1/api/jobs` | List jobs (paginated) |
| `GET` | `/v1/api/quota` | Check remaining quota |

## License

MIT
