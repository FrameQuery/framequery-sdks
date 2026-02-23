# FrameQuery Python SDK

Official Python client for the [FrameQuery](https://framequery.com) video processing API.

## Installation

```bash
pip install framequery
```

Requires Python 3.9+.

## Quick Start

```python
from framequery import FrameQuery

fq = FrameQuery(api_key="fq_...")

# Process a video — uploads and waits for results in one call
result = fq.process("interview.mp4")

print(f"Duration: {result.duration}s")
for scene in result.scenes:
    print(f"  [{scene.end_time}s] {scene.description}")
for seg in result.transcript:
    print(f"  [{seg.start_time}-{seg.end_time}s] {seg.text}")
```

## Process from URL

```python
result = fq.process_url("https://cdn.example.com/video.mp4")
```

## Upload Without Waiting

```python
job = fq.upload("video.mp4")
print(job.id)  # available immediately

# Check back later
job = fq.get_job(job.id)
if job.is_complete:
    print("Done!")
```

## Progress Tracking

```python
def on_progress(job):
    print(f"Status: {job.status}, ETA: {job.eta_seconds}s")

result = fq.process("video.mp4", on_progress=on_progress)
```

## Async Support

```python
from framequery import AsyncFrameQuery

async with AsyncFrameQuery(api_key="fq_...") as fq:
    result = await fq.process("video.mp4")
    print(result.scenes)
```

## Check Quota

```python
quota = fq.get_quota()
print(f"{quota.plan}: {quota.credits_balance_hours}h credits remaining")
```

## List Jobs

```python
page = fq.list_jobs(limit=10, status="COMPLETED")
for job in page.jobs:
    print(f"{job.id}: {job.filename}")
if page.has_more:
    next_page = fq.list_jobs(cursor=page.next_cursor)
```

## Configuration

```python
fq = FrameQuery(
    api_key="fq_...",                          # or set FRAMEQUERY_API_KEY env var
    base_url="https://api.framequery.com/v1/api",  # default
    timeout=300.0,                              # HTTP timeout in seconds
    max_retries=2,                              # retries on 5xx/network errors
)
```

## Error Handling

```python
from framequery import (
    FrameQueryError,
    AuthenticationError,
    NotFoundError,
    RateLimitError,
    JobFailedError,
)

try:
    result = fq.process("video.mp4")
except AuthenticationError:
    print("Invalid API key")
except RateLimitError as e:
    print(f"Rate limited, retry after {e.retry_after}s")
except JobFailedError as e:
    print(f"Job {e.job_id} failed")
except FrameQueryError as e:
    print(f"API error: {e.message}")
```

## License

MIT
