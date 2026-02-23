# FrameQuery Go SDK

Go client for the [FrameQuery API](https://framequery.com). Upload videos, poll for results, list jobs, check quota.

```bash
go get github.com/framequery/framequery-go
```

Go 1.22+. No external dependencies.

## Usage

```go
client := framequery.New("fq_...")
result, err := client.Process(ctx, "interview.mp4", nil)
if err != nil {
    log.Fatal(err)
}

for _, s := range result.Scenes {
    fmt.Printf("[%.1fs] %s\n", s.EndTime, s.Description)
}
for _, t := range result.Transcript {
    fmt.Printf("[%.1f-%.1fs] %s\n", t.StartTime, t.EndTime, t.Text)
}
```

### Process from URL

```go
result, err := client.ProcessURL(ctx, "https://cdn.example.com/video.mp4", nil)
```

### Upload without waiting

```go
job, err := client.Upload(ctx, "video.mp4", nil)
// ...later
job, err = client.GetJob(ctx, job.ID)
if job.IsComplete() { /* ... */ }
```

### Progress callback

```go
result, err := client.Process(ctx, "video.mp4", &framequery.ProcessOptions{
    OnProgress: func(j *framequery.Job) {
        fmt.Printf("%s (ETA: %.0fs)\n", j.Status, j.ETASeconds)
    },
})
```

### Client options

```go
client := framequery.New("fq_...",
    framequery.WithBaseURL("https://custom.api.com/v1/api"),
    framequery.WithMaxRetries(3),   // default 2
    framequery.WithTimeout(10*time.Minute), // default 5m per request
    framequery.WithHTTPClient(customClient),
)
```

### Error handling

```go
_, err := client.GetJob(ctx, "bad-id")
if framequery.IsNotFoundError(err) {
    // 404
} else if framequery.IsAuthError(err) {
    // 401
} else if framequery.IsRateLimitError(err) {
    // 429 — retries are automatic, so this means retries were exhausted
}
```

### Quota

```go
q, _ := client.GetQuota(ctx)
fmt.Printf("%s: %.1fh credits left\n", q.Plan, q.CreditsBalanceHours)
```

### List jobs (cursor pagination)

```go
page, _ := client.ListJobs(ctx, &framequery.ListJobsOptions{
    Limit:  10,
    Status: "COMPLETED",
})
for _, j := range page.Jobs {
    fmt.Println(j.ID, j.Filename)
}
if page.HasMore() {
    next, _ := client.ListJobs(ctx, &framequery.ListJobsOptions{Cursor: page.NextCursor})
}
```

## License

MIT
