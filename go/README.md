# FrameQuery Go SDK

Official Go client for the [FrameQuery](https://framequery.com) video processing API.

## Installation

```bash
go get github.com/framequery/framequery-go
```

Requires Go 1.22+. Zero external dependencies.

## Quick Start

```go
package main

import (
    "context"
    "fmt"
    "log"

    framequery "github.com/framequery/framequery-go"
)

func main() {
    client := framequery.New("fq_...")
    ctx := context.Background()

    result, err := client.Process(ctx, "interview.mp4", nil)
    if err != nil {
        log.Fatal(err)
    }

    fmt.Printf("Duration: %.1fs\n", result.Duration)
    for _, s := range result.Scenes {
        fmt.Printf("  [%.1fs] %s\n", s.EndTime, s.Description)
    }
    for _, t := range result.Transcript {
        fmt.Printf("  [%.1f-%.1fs] %s\n", t.StartTime, t.EndTime, t.Text)
    }
}
```

## Process from URL

```go
result, err := client.ProcessURL(ctx, "https://cdn.example.com/video.mp4", nil)
```

## Upload Without Waiting

```go
job, err := client.Upload(ctx, "video.mp4", nil)
fmt.Println(job.ID) // available immediately

// Check back later
job, err = client.GetJob(ctx, job.ID)
if job.IsComplete() {
    fmt.Println("Done!")
}
```

## Progress Tracking

```go
result, err := client.Process(ctx, "video.mp4", &framequery.ProcessOptions{
    OnProgress: func(j *framequery.Job) {
        fmt.Printf("Status: %s, ETA: %.0fs\n", j.Status, j.ETASeconds)
    },
})
```

## Functional Options

```go
client := framequery.New("fq_...",
    framequery.WithBaseURL("https://custom.api.com/v1/api"),
    framequery.WithMaxRetries(3),
    framequery.WithTimeout(10 * time.Minute),
    framequery.WithHTTPClient(customClient),
)
```

## Error Handling

```go
job, err := client.GetJob(ctx, "invalid-id")
if framequery.IsNotFoundError(err) {
    fmt.Println("Job not found")
} else if framequery.IsAuthError(err) {
    fmt.Println("Invalid API key")
} else if framequery.IsRateLimitError(err) {
    fmt.Println("Rate limited")
} else if err != nil {
    fmt.Println("Error:", err)
}
```

## Check Quota

```go
quota, err := client.GetQuota(ctx)
fmt.Printf("%s: %.1fh credits remaining\n", quota.Plan, quota.CreditsBalanceHours)
```

## List Jobs

```go
page, err := client.ListJobs(ctx, &framequery.ListJobsOptions{
    Limit:  10,
    Status: "COMPLETED",
})
for _, j := range page.Jobs {
    fmt.Printf("%s: %s\n", j.ID, j.Filename)
}
if page.HasMore() {
    nextPage, _ := client.ListJobs(ctx, &framequery.ListJobsOptions{
        Cursor: page.NextCursor,
    })
}
```

## License

MIT
