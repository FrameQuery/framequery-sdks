# FrameQuery TypeScript SDK

Official TypeScript/JavaScript client for the [FrameQuery](https://framequery.com) video processing API.

Works in Node.js 18+ and modern browsers.

## Installation

```bash
npm install framequery
```

## Quick Start

```typescript
import FrameQuery from "framequery";

const fq = new FrameQuery({ apiKey: "fq_..." });

const result = await fq.process("./interview.mp4");

console.log(`Duration: ${result.duration}s`);
result.scenes.forEach((s) => console.log(`  [${s.endTime}s] ${s.description}`));
result.transcript.forEach((t) => console.log(`  [${t.startTime}-${t.endTime}s] ${t.text}`));
```

## Browser Usage

```typescript
// Pass a File or Blob instead of a path
const result = await fq.process(fileInput.files[0], { filename: "video.mp4" });
```

## Process from URL

```typescript
const result = await fq.processUrl("https://cdn.example.com/video.mp4");
```

## Upload Without Waiting

```typescript
const job = await fq.upload("./video.mp4");
console.log(job.id); // available immediately

// Check back later
const updated = await fq.getJob(job.id);
if (updated.isComplete) console.log("Done!");
```

## Progress Tracking

```typescript
const result = await fq.process("./video.mp4", {
  onProgress: (job) => {
    console.log(`Status: ${job.status}, ETA: ${job.etaSeconds}s`);
  },
});
```

## Check Quota

```typescript
const quota = await fq.getQuota();
console.log(`${quota.plan}: ${quota.creditsBalanceHours}h credits remaining`);
```

## List Jobs

```typescript
const page = await fq.listJobs({ limit: 10, status: "COMPLETED" });
for (const job of page.jobs) {
  console.log(`${job.id}: ${job.filename}`);
}
if (page.hasMore) {
  const next = await fq.listJobs({ cursor: page.nextCursor! });
}
```

## Configuration

```typescript
const fq = new FrameQuery({
  apiKey: "fq_...",                              // or set FRAMEQUERY_API_KEY env
  baseUrl: "https://api.framequery.com/v1/api",  // default
  timeout: 300_000,                               // HTTP timeout in ms
  maxRetries: 2,                                  // retries on 5xx/network errors
  fetch: customFetch,                             // custom fetch implementation
});
```

## Error Handling

```typescript
import FrameQuery, {
  AuthenticationError,
  NotFoundError,
  RateLimitError,
  JobFailedError,
} from "framequery";

try {
  const result = await fq.process("./video.mp4");
} catch (err) {
  if (err instanceof AuthenticationError) {
    console.log("Invalid API key");
  } else if (err instanceof RateLimitError) {
    console.log(`Rate limited, retry after ${err.retryAfter}s`);
  } else if (err instanceof JobFailedError) {
    console.log(`Job ${err.jobId} failed`);
  }
}
```

## License

MIT
