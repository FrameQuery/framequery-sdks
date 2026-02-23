package framequery

import "time"

// Scene is a single detected scene with a description, end timestamp, and tagged objects.
type Scene struct {
	Description string   `json:"description"`
	EndTime     float64  `json:"endTs"`
	Objects     []string `json:"objects"`
}

// TranscriptSegment is one timed chunk of the speech-to-text transcript.
type TranscriptSegment struct {
	StartTime float64 `json:"StartTime"`
	EndTime   float64 `json:"EndTime"`
	Text      string  `json:"Text"`
}

// ProcessedData maps to the processedData field in the job JSON.
type ProcessedData struct {
	Length     float64             `json:"length"`
	Scenes     []Scene            `json:"scenes"`
	Transcript []TranscriptSegment `json:"transcript"`
}

// ProcessingResult is returned when a job reaches a terminal success state.
type ProcessingResult struct {
	JobID      string
	Status     string
	Filename   string
	Duration   float64
	Scenes     []Scene
	Transcript []TranscriptSegment
	CreatedAt  string
	Raw        map[string]any
}

// Job tracks a video through the processing pipeline. Raw holds the full API response.
type Job struct {
	ID         string
	Status     string
	Filename   string
	CreatedAt  string
	ETASeconds float64
	Raw        map[string]any
}

// IsTerminal reports whether the job is done (COMPLETED, COMPLETED_NO_SCENES, or FAILED).
func (j *Job) IsTerminal() bool {
	return j.Status == "COMPLETED" || j.Status == "COMPLETED_NO_SCENES" || j.Status == "FAILED"
}

// IsComplete reports whether the job finished successfully (COMPLETED or COMPLETED_NO_SCENES).
func (j *Job) IsComplete() bool {
	return j.Status == "COMPLETED" || j.Status == "COMPLETED_NO_SCENES"
}

// IsFailed reports whether the job status is FAILED.
func (j *Job) IsFailed() bool {
	return j.Status == "FAILED"
}

// Quota holds the account's plan, included hours, credit balance, and reset date.
type Quota struct {
	Plan                string  `json:"plan"`
	IncludedHours       float64 `json:"includedHours"`
	CreditsBalanceHours float64 `json:"creditsBalanceHours"`
	ResetDate           string  `json:"resetDate"`
}

// JobPage is one page from ListJobs. Use NextCursor to fetch the next page.
type JobPage struct {
	Jobs       []Job
	NextCursor string
}

// HasMore reports whether another page is available.
func (p *JobPage) HasMore() bool {
	return p.NextCursor != ""
}

// ProcessOptions tunes polling behavior for Process and ProcessURL.
// Defaults: 5s poll interval, 24h timeout.
type ProcessOptions struct {
	PollInterval time.Duration
	Timeout      time.Duration
	OnProgress   func(*Job)
}

// UploadOptions overrides the filename derived from the file path.
type UploadOptions struct {
	Filename string
}

// ListJobsOptions filters and paginates ListJobs.
type ListJobsOptions struct {
	Limit  int
	Cursor string
	Status string
}

// ---- Internal API response types ----

type apiEnvelope struct {
	Data       any    `json:"data"`
	NextCursor string `json:"nextCursor,omitempty"`
}

type createJobResponse struct {
	JobID        string `json:"jobId"`
	UploadURL    string `json:"uploadUrl"`
	ExpiresIn    int    `json:"expiresInSeconds"`
	UploadMethod string `json:"uploadMethod"`
	Status       string `json:"status,omitempty"`
}

type createJobFromURLResponse struct {
	JobID  string `json:"jobId"`
	Status string `json:"status"`
}

func parseJob(data map[string]any) *Job {
	j := &Job{Raw: data}
	if v, ok := data["jobId"].(string); ok {
		j.ID = v
	}
	if v, ok := data["status"].(string); ok {
		j.Status = v
	}
	if v, ok := data["originalFilename"].(string); ok {
		j.Filename = v
	}
	if v, ok := data["createdAt"].(string); ok {
		j.CreatedAt = v
	}
	if v, ok := data["estimatedCompletionTimeSeconds"].(float64); ok {
		j.ETASeconds = v
	}
	return j
}

func parseResult(data map[string]any) *ProcessingResult {
	r := &ProcessingResult{Raw: data}
	if v, ok := data["jobId"].(string); ok {
		r.JobID = v
	}
	if v, ok := data["status"].(string); ok {
		r.Status = v
	}
	if v, ok := data["originalFilename"].(string); ok {
		r.Filename = v
	}
	if v, ok := data["createdAt"].(string); ok {
		r.CreatedAt = v
	}

	if pd, ok := data["processedData"].(map[string]any); ok {
		if v, ok := pd["length"].(float64); ok {
			r.Duration = v
		}
		if scenes, ok := pd["scenes"].([]any); ok {
			for _, s := range scenes {
				if sm, ok := s.(map[string]any); ok {
					scene := Scene{}
					if v, ok := sm["description"].(string); ok {
						scene.Description = v
					}
					if v, ok := sm["endTs"].(float64); ok {
						scene.EndTime = v
					}
					if objs, ok := sm["objects"].([]any); ok {
						for _, o := range objs {
							if str, ok := o.(string); ok {
								scene.Objects = append(scene.Objects, str)
							}
						}
					}
					r.Scenes = append(r.Scenes, scene)
				}
			}
		}
		if transcript, ok := pd["transcript"].([]any); ok {
			for _, t := range transcript {
				if tm, ok := t.(map[string]any); ok {
					seg := TranscriptSegment{}
					if v, ok := tm["StartTime"].(float64); ok {
						seg.StartTime = v
					}
					if v, ok := tm["EndTime"].(float64); ok {
						seg.EndTime = v
					}
					if v, ok := tm["Text"].(string); ok {
						seg.Text = v
					}
					r.Transcript = append(r.Transcript, seg)
				}
			}
		}
	}
	return r
}
