# framequery

Ruby SDK for the [FrameQuery](https://framequery.com) API. Upload videos, poll jobs, get scenes and transcripts back.

Ruby 3.0+. No runtime dependencies.

## Install

```ruby
gem "framequery"
```

## Usage

```ruby
require "framequery"

client = FrameQuery::Client.new(api_key: "fq_...")
# or set FRAMEQUERY_API_KEY and omit api_key

result = client.process("interview.mp4")
result.scenes.each { |s| puts "#{s.end_time}s: #{s.description}" }
result.transcript.each { |t| puts "[#{t.start_time}-#{t.end_time}s] #{t.text}" }
```

### From URL

```ruby
result = client.process_url("https://cdn.example.com/video.mp4")
```

### Upload without waiting

```ruby
job = client.upload("video.mp4")
# ... later
job = client.get_job(job.id)
puts "done" if job.complete?
```

### Progress callback

```ruby
result = client.process("video.mp4") { |job|
  puts "#{job.status} eta=#{job.eta_seconds}s"
}
```

### Pagination

```ruby
page = client.list_jobs(limit: 10, status: "COMPLETED")
page.jobs.each { |j| puts "#{j.id}: #{j.filename}" }
next_page = client.list_jobs(cursor: page.next_cursor) if page.more?
```

### Quota

```ruby
q = client.get_quota
puts "#{q.plan}: #{q.credits_balance_hours}h credits, #{q.included_hours}h included"
```

## Configuration

| Param | Default | Notes |
|---|---|---|
| `api_key` | `ENV["FRAMEQUERY_API_KEY"]` | Required |
| `base_url` | `https://api.framequery.com/v1/api` | |
| `timeout` | `300` | Per-request HTTP timeout (seconds) |
| `max_retries` | `2` | Retries on 5xx, 429, and network errors. Exponential backoff, honors `Retry-After`. |

## Errors

All errors inherit from `FrameQuery::Error`.

| Class | When |
|---|---|
| `AuthenticationError` | 401 |
| `PermissionDeniedError` | 403 |
| `NotFoundError` | 404 |
| `RateLimitError` | 429 -- check `.retry_after` |
| `APIError` | Other non-2xx -- has `.status_code` and `.body` |
| `JobFailedError` | Job status became FAILED during polling |
| `TimeoutError` | Polling exceeded timeout |

## License

MIT
