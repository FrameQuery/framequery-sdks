# FrameQuery Ruby SDK

Official Ruby client for the [FrameQuery](https://framequery.com) video processing API.

## Installation

```ruby
gem "framequery"
```

Or install directly:

```bash
gem install framequery
```

Requires Ruby 3.0+. Zero runtime dependencies (uses stdlib `net/http`).

## Quick Start

```ruby
require "framequery"

client = FrameQuery::Client.new(api_key: "fq_...")

result = client.process("interview.mp4")

puts "Duration: #{result.duration}s"
result.scenes.each { |s| puts "  [#{s.end_time}s] #{s.description}" }
result.transcript.each { |t| puts "  [#{t.start_time}-#{t.end_time}s] #{t.text}" }
```

## Process from URL

```ruby
result = client.process_url("https://cdn.example.com/video.mp4")
```

## Upload Without Waiting

```ruby
job = client.upload("video.mp4")
puts job.id  # available immediately

# Check back later
job = client.get_job(job.id)
puts "Done!" if job.complete?
```

## Progress Tracking

```ruby
result = client.process("video.mp4") { |job|
  puts "Status: #{job.status}, ETA: #{job.eta_seconds}s"
}
```

## Check Quota

```ruby
quota = client.get_quota
puts "#{quota.plan}: #{quota.credits_balance_hours}h credits remaining"
```

## List Jobs

```ruby
page = client.list_jobs(limit: 10, status: "COMPLETED")
page.jobs.each { |j| puts "#{j.id}: #{j.filename}" }
if page.more?
  next_page = client.list_jobs(cursor: page.next_cursor)
end
```

## Configuration

```ruby
client = FrameQuery::Client.new(
  api_key: "fq_...",                              # or set FRAMEQUERY_API_KEY env
  base_url: "https://api.framequery.com/v1/api",  # default
  timeout: 300,                                    # HTTP timeout in seconds
  max_retries: 2                                   # retries on 5xx/network errors
)
```

## Error Handling

```ruby
begin
  result = client.process("video.mp4")
rescue FrameQuery::AuthenticationError
  puts "Invalid API key"
rescue FrameQuery::RateLimitError => e
  puts "Rate limited, retry after #{e.retry_after}s"
rescue FrameQuery::JobFailedError => e
  puts "Job #{e.job_id} failed"
rescue FrameQuery::Error => e
  puts "API error: #{e.message}"
end
```

## License

MIT
