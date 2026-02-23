# FrameQuery SDKs

Client libraries for the [FrameQuery](https://framequery.com) video processing API. Upload a video, get back scenes and transcripts.

## Languages

| Language | Package | Install |
|---|---|---|
| [Python](./python) | `framequery` | `pip install framequery` |
| [TypeScript / Node.js](./typescript) | `framequery` | `npm install framequery` |
| [Go](./go) | `framequery-go` | `go get github.com/framequery/framequery-go` |
| [Rust](./rust) | `framequery` | `cargo add framequery` |
| [Ruby](./ruby) | `framequery` | `gem install framequery` |

## Quick Start

Same pattern everywhere — create a client, call `process()`, get typed results back.

### Python

```python
from framequery import FrameQuery

fq = FrameQuery(api_key="fq_...")

result = fq.process("interview.mp4")
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
result.scenes.forEach((s) => console.log(`[${s.endTime}s] ${s.description}`));
```

### Go

```go
client := framequery.New("fq_...")

result, err := client.Process(ctx, "interview.mp4", nil)
if err != nil {
    log.Fatal(err)
}
for _, s := range result.Scenes {
    fmt.Printf("[%.1fs] %s\n", s.EndTime, s.Description)
}
```

### Rust

```rust
let client = framequery::Client::new("fq_...");

let result = client.process("interview.mp4", None).await?;
for scene in &result.scenes {
    println!("[{}s] {}", scene.end_time, scene.description);
}
```

### Ruby

```ruby
client = FrameQuery::Client.new(api_key: "fq_...")

result = client.process("interview.mp4")
result.scenes.each { |s| puts "[#{s.end_time}s] #{s.description}" }
```

## Auth

Grab an API key from the [dashboard](https://app.framequery.com/settings/api-keys). Every SDK also checks `FRAMEQUERY_API_KEY` from the environment:

```bash
export FRAMEQUERY_API_KEY=fq_...
```

## Other Things You Can Do

```python
# Process from a URL instead of a local file
result = fq.process_url("https://cdn.example.com/video.mp4")

# Upload without blocking — returns a job you can poll later
job = fq.upload("video.mp4")
print(job.id)

# Progress callback
result = fq.process("video.mp4", on_progress=lambda j: print(j.status))

# Check your quota
quota = fq.get_quota()
print(f"{quota.credits_balance_hours}h remaining")
```

## Endpoints

| Method | Endpoint | What it does |
|---|---|---|
| `POST` | `/v1/api/jobs` | Create a job, get a signed upload URL back |
| `POST` | `/v1/api/jobs/from-url` | Create a job from a remote URL |
| `GET` | `/v1/api/jobs/{jobId}` | Get job status and results |
| `GET` | `/v1/api/jobs` | List jobs (cursor-paginated) |
| `GET` | `/v1/api/quota` | Check remaining hours |

## License

MIT
