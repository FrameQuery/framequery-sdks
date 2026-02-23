package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"time"

	framequery "github.com/framequery/framequery-go"
)

func main() {
	client := framequery.New(os.Getenv("FRAMEQUERY_API_KEY"))
	ctx := context.Background()

	// 1. Process a local file (upload + wait)
	result, err := client.Process(ctx, "interview.mp4", &framequery.ProcessOptions{
		OnProgress: func(j *framequery.Job) {
			fmt.Printf("  Status: %s (ETA: %.0fs)\n", j.Status, j.ETASeconds)
		},
	})
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Duration: %.1fs\n", result.Duration)
	for _, s := range result.Scenes {
		fmt.Printf("  [%.1fs] %s — %v\n", s.EndTime, s.Description, s.Objects)
	}
	for _, t := range result.Transcript {
		fmt.Printf("  [%.1f-%.1fs] %s\n", t.StartTime, t.EndTime, t.Text)
	}

	// 2. Process from URL
	urlResult, err := client.ProcessURL(ctx, "https://cdn.example.com/video.mp4", nil)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("URL video: %.1fs, %d scenes\n", urlResult.Duration, len(urlResult.Scenes))

	// 3. Upload without waiting
	job, err := client.Upload(ctx, "video.mp4", nil)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Job created: %s (%s)\n", job.ID, job.Status)

	// Check back later
	job, err = client.GetJob(ctx, job.ID)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Job status: %s\n", job.Status)

	// 4. Check quota
	quota, err := client.GetQuota(ctx)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Plan: %s, Credits: %.1fh remaining\n", quota.Plan, quota.CreditsBalanceHours)

	// 5. List jobs
	page, err := client.ListJobs(ctx, &framequery.ListJobsOptions{Limit: 10, Status: "COMPLETED"})
	if err != nil {
		log.Fatal(err)
	}
	for _, j := range page.Jobs {
		fmt.Printf("  %s: %s — %s\n", j.ID, j.Status, j.Filename)
	}
	if page.HasMore() {
		fmt.Printf("  ... more available (cursor: %s)\n", page.NextCursor)
	}

	_ = time.Second // avoid unused import
}
