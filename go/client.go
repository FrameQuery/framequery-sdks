// Package framequery wraps the FrameQuery API (upload, process, poll, list jobs).
//
//	client := framequery.New("fq_your_api_key")
//	result, err := client.Process(ctx, "video.mp4", nil)
//	fmt.Println(result.Scenes)
package framequery

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"math"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"strconv"
	"time"
)

const (
	defaultBaseURL      = "https://api.framequery.com/v1/api"
	defaultPollInterval = 5 * time.Second
	defaultTimeout      = 24 * time.Hour
	defaultMaxRetries   = 2
	defaultHTTPTimeout  = 5 * time.Minute
	version             = "0.1.0"
)

// Client holds auth credentials and HTTP configuration for API calls.
type Client struct {
	baseURL    string
	apiKey     string
	httpClient *http.Client
	maxRetries int
}

// Option is a functional option for New.
type Option func(*Client)

// WithBaseURL overrides the default API endpoint (https://api.framequery.com/v1/api).
func WithBaseURL(u string) Option {
	return func(c *Client) { c.baseURL = u }
}

// WithHTTPClient replaces the default http.Client (5m timeout).
func WithHTTPClient(hc *http.Client) Option {
	return func(c *Client) { c.httpClient = hc }
}

// WithMaxRetries sets retry count for 5xx and 429 responses (default 2).
func WithMaxRetries(n int) Option {
	return func(c *Client) { c.maxRetries = n }
}

// WithTimeout sets the per-request HTTP timeout (default 5m).
func WithTimeout(d time.Duration) Option {
	return func(c *Client) { c.httpClient.Timeout = d }
}

// New creates a Client. Falls back to FRAMEQUERY_API_KEY env var if apiKey is empty.
func New(apiKey string, opts ...Option) *Client {
	if apiKey == "" {
		apiKey = os.Getenv("FRAMEQUERY_API_KEY")
	}
	c := &Client{
		baseURL:    defaultBaseURL,
		apiKey:     apiKey,
		maxRetries: defaultMaxRetries,
		httpClient: &http.Client{Timeout: defaultHTTPTimeout},
	}
	for _, opt := range opts {
		opt(c)
	}
	return c
}

// Process uploads a video file from disk and blocks until the job finishes or fails.
func (c *Client) Process(ctx context.Context, path string, opts *ProcessOptions) (*ProcessingResult, error) {
	job, err := c.Upload(ctx, path, nil)
	if err != nil {
		return nil, err
	}
	return c.poll(ctx, job.ID, opts)
}

// ProcessURL submits a remote video URL and blocks until the job finishes or fails.
func (c *Client) ProcessURL(ctx context.Context, videoURL string, opts *ProcessOptions) (*ProcessingResult, error) {
	body := map[string]string{"url": videoURL}
	var resp createJobFromURLResponse
	if err := c.doJSON(ctx, http.MethodPost, "/jobs/from-url", body, &resp); err != nil {
		return nil, err
	}
	return c.poll(ctx, resp.JobID, opts)
}

// Upload sends a video file and returns the Job without waiting for processing.
func (c *Client) Upload(ctx context.Context, path string, opts *UploadOptions) (*Job, error) {
	filename := filepath.Base(path)
	if opts != nil && opts.Filename != "" {
		filename = opts.Filename
	}

	// Create job
	var resp createJobResponse
	if err := c.doJSON(ctx, http.MethodPost, "/jobs", map[string]string{"fileName": filename}, &resp); err != nil {
		return nil, err
	}

	// Upload file to signed URL
	f, err := os.Open(path)
	if err != nil {
		return nil, fmt.Errorf("framequery: open file: %w", err)
	}
	defer f.Close()

	req, err := http.NewRequestWithContext(ctx, http.MethodPut, resp.UploadURL, f)
	if err != nil {
		return nil, fmt.Errorf("framequery: create upload request: %w", err)
	}
	req.Header.Set("Content-Type", "application/octet-stream")

	uploadResp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("framequery: upload: %w", err)
	}
	defer uploadResp.Body.Close()

	if uploadResp.StatusCode < 200 || uploadResp.StatusCode >= 300 {
		b, _ := io.ReadAll(uploadResp.Body)
		return nil, fmt.Errorf("framequery: upload failed %s: %s", uploadResp.Status, string(b))
	}

	return &Job{
		ID:       resp.JobID,
		Status:   "PENDING_UPLOAD",
		Filename: filename,
		Raw:      map[string]any{"jobId": resp.JobID, "status": "PENDING_UPLOAD"},
	}, nil
}

// GetJob returns a job's current status and results.
func (c *Client) GetJob(ctx context.Context, jobID string) (*Job, error) {
	var raw map[string]any
	if err := c.doJSON(ctx, http.MethodGet, "/jobs/"+url.PathEscape(jobID), nil, &raw); err != nil {
		return nil, err
	}
	return parseJob(raw), nil
}

// ListJobs returns a page of jobs. Supports cursor pagination and status filtering.
func (c *Client) ListJobs(ctx context.Context, opts *ListJobsOptions) (*JobPage, error) {
	path := "/jobs"
	params := url.Values{}
	if opts != nil {
		if opts.Limit > 0 {
			params.Set("limit", strconv.Itoa(opts.Limit))
		}
		if opts.Cursor != "" {
			params.Set("cursor", opts.Cursor)
		}
		if opts.Status != "" {
			params.Set("status", opts.Status)
		}
	}
	if len(params) > 0 {
		path += "?" + params.Encode()
	}

	// List returns {"data": [...], "nextCursor": "..."} so we need raw response
	raw, err := c.doJSONRaw(ctx, http.MethodGet, path, nil)
	if err != nil {
		return nil, err
	}

	page := &JobPage{}
	if cursor, ok := raw["nextCursor"].(string); ok {
		page.NextCursor = cursor
	}
	if items, ok := raw["data"].([]any); ok {
		for _, item := range items {
			if m, ok := item.(map[string]any); ok {
				page.Jobs = append(page.Jobs, *parseJob(m))
			}
		}
	}
	return page, nil
}

// GetQuota returns included hours, credit balance, and plan info.
func (c *Client) GetQuota(ctx context.Context) (*Quota, error) {
	var q Quota
	if err := c.doJSON(ctx, http.MethodGet, "/quota", nil, &q); err != nil {
		return nil, err
	}
	return &q, nil
}

// ---- Private ----

func (c *Client) poll(ctx context.Context, jobID string, opts *ProcessOptions) (*ProcessingResult, error) {
	interval := defaultPollInterval
	timeout := defaultTimeout
	var onProgress func(*Job)

	if opts != nil {
		if opts.PollInterval > 0 {
			interval = opts.PollInterval
		}
		if opts.Timeout > 0 {
			timeout = opts.Timeout
		}
		onProgress = opts.OnProgress
	}

	ctx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()

	ticker := time.NewTicker(interval)
	defer ticker.Stop()

	for {
		job, err := c.GetJob(ctx, jobID)
		if err != nil {
			return nil, err
		}

		if onProgress != nil {
			onProgress(job)
		}

		if job.IsFailed() {
			msg, _ := job.Raw["errorMessage"].(string)
			return nil, &Error{Message: fmt.Sprintf("job %s failed: %s", jobID, msg)}
		}

		if job.IsComplete() {
			return parseResult(job.Raw), nil
		}

		// Adaptive interval
		currentInterval := interval
		if job.ETASeconds > 60 {
			adaptive := time.Duration(job.ETASeconds/3) * time.Second
			if adaptive > 30*time.Second {
				adaptive = 30 * time.Second
			}
			currentInterval = adaptive
			ticker.Reset(currentInterval)
		}
		_ = currentInterval

		select {
		case <-ctx.Done():
			return nil, fmt.Errorf("framequery: timed out waiting for job %s: %w", jobID, ctx.Err())
		case <-ticker.C:
		}
	}
}

// doJSON makes an API request, unwraps the {"data": ...} envelope, and decodes into out.
func (c *Client) doJSON(ctx context.Context, method, path string, body any, out any) error {
	raw, err := c.doJSONRaw(ctx, method, path, body)
	if err != nil {
		return err
	}

	// Unwrap "data" envelope
	dataVal, hasData := raw["data"]
	if !hasData {
		// No envelope, decode entire response
		b, _ := json.Marshal(raw)
		return json.Unmarshal(b, out)
	}

	b, err := json.Marshal(dataVal)
	if err != nil {
		return fmt.Errorf("framequery: marshal data: %w", err)
	}
	return json.Unmarshal(b, out)
}

// doJSONRaw makes an API request and returns the raw JSON response. Retries on 5xx/429.
func (c *Client) doJSONRaw(ctx context.Context, method, path string, body any) (map[string]any, error) {
	apiURL := c.baseURL + path

	var bodyReader io.Reader
	if body != nil {
		b, err := json.Marshal(body)
		if err != nil {
			return nil, fmt.Errorf("framequery: marshal body: %w", err)
		}
		bodyReader = bytes.NewReader(b)
	}

	var lastErr error
	for attempt := 0; attempt <= c.maxRetries; attempt++ {
		req, err := http.NewRequestWithContext(ctx, method, apiURL, bodyReader)
		if err != nil {
			return nil, fmt.Errorf("framequery: create request: %w", err)
		}
		req.Header.Set("Authorization", "Bearer "+c.apiKey)
		req.Header.Set("User-Agent", "framequery-go/"+version)
		if body != nil {
			req.Header.Set("Content-Type", "application/json")
		}

		resp, err := c.httpClient.Do(req)
		if err != nil {
			lastErr = err
			if attempt < c.maxRetries {
				time.Sleep(backoff(attempt))
				// Reset body reader for retry
				if body != nil {
					b, _ := json.Marshal(body)
					bodyReader = bytes.NewReader(b)
				}
				continue
			}
			return nil, fmt.Errorf("framequery: request failed: %w", err)
		}
		defer resp.Body.Close()

		respBody, err := io.ReadAll(resp.Body)
		if err != nil {
			return nil, fmt.Errorf("framequery: read response: %w", err)
		}

		if resp.StatusCode >= 500 || resp.StatusCode == 429 {
			if attempt < c.maxRetries {
				delay := backoff(attempt)
				if ra := resp.Header.Get("Retry-After"); ra != "" {
					if secs, err := strconv.ParseFloat(ra, 64); err == nil {
						delay = time.Duration(secs * float64(time.Second))
					}
				}
				time.Sleep(delay)
				if body != nil {
					b, _ := json.Marshal(body)
					bodyReader = bytes.NewReader(b)
				}
				continue
			}
		}

		if resp.StatusCode < 200 || resp.StatusCode >= 300 {
			apiErr := &Error{StatusCode: resp.StatusCode}
			var errBody map[string]any
			if json.Unmarshal(respBody, &errBody) == nil {
				if msg, ok := errBody["error"].(string); ok {
					apiErr.Message = msg
				} else if msg, ok := errBody["message"].(string); ok {
					apiErr.Message = msg
				}
				apiErr.Body = errBody
			}
			if apiErr.Message == "" {
				apiErr.Message = string(respBody)
			}
			return nil, apiErr
		}

		var result map[string]any
		if err := json.Unmarshal(respBody, &result); err != nil {
			return nil, fmt.Errorf("framequery: unmarshal response: %w", err)
		}
		return result, nil
	}

	if lastErr != nil {
		return nil, fmt.Errorf("framequery: request failed after retries: %w", lastErr)
	}
	return nil, fmt.Errorf("framequery: request failed")
}

func backoff(attempt int) time.Duration {
	ms := 500.0 * math.Pow(2, float64(attempt))
	if ms > 30000 {
		ms = 30000
	}
	return time.Duration(ms) * time.Millisecond
}
