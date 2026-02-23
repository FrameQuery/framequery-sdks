# framequery

Python SDK for [FrameQuery](https://framequery.com) -- extract scenes and transcripts from video.

## Install

```bash
pip install framequery
```

Python 3.9+

## Usage

```python
from framequery import FrameQuery

fq = FrameQuery(api_key="fq_...")

result = fq.process("interview.mp4")

for scene in result.scenes:
    print(f"[{scene.end_time}s] {scene.description}")
for seg in result.transcript:
    print(f"[{seg.start_time}-{seg.end_time}s] {seg.text}")
```

### From URL

```python
result = fq.process_url("https://cdn.example.com/video.mp4")
```

### Upload only (don't wait)

```python
job = fq.upload("video.mp4")
# ...
job = fq.get_job(job.id)
```

### Progress callback

```python
result = fq.process("video.mp4", on_progress=lambda j: print(j.status))
```

### Async

```python
from framequery import AsyncFrameQuery

async with AsyncFrameQuery(api_key="fq_...") as fq:
    result = await fq.process("video.mp4")
```

### Pagination

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
    api_key="fq_...",       # or FRAMEQUERY_API_KEY env var
    timeout=300.0,           # HTTP timeout (seconds), default 300
    max_retries=2,           # retries on 5xx / network errors, default 2
)
```

## Errors

All errors inherit from `FrameQueryError`.

```python
from framequery import AuthenticationError, RateLimitError, JobFailedError

try:
    result = fq.process("video.mp4")
except RateLimitError as e:
    print(f"retry after {e.retry_after}s")
except JobFailedError as e:
    print(f"job {e.job_id} failed")
```

## License

MIT
