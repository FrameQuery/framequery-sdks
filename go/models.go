package framequery

import (
	"strings"
	"time"
)

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

// AudioTrack describes an additional audio track attached to a job.
type AudioTrack struct {
	FileName              string `json:"fileName"`
	URL                   string `json:"url,omitempty"`
	DownloadToken         string `json:"downloadToken,omitempty"`
	SyncMode              string `json:"syncMode,omitempty"`
	OffsetMs              int    `json:"offsetMs,omitempty"`
	Label                 string `json:"label,omitempty"`
	PerChannelTranscription bool `json:"perChannelTranscription,omitempty"`
	Channels              int    `json:"channels,omitempty"`
}

// AudioTrackTranscript holds the transcript result for a single audio track.
type AudioTrackTranscript struct {
	TrackIndex   int                 `json:"trackIndex"`
	TrackName    string              `json:"trackName"`
	Language     string              `json:"language"`
	Status       string              `json:"status"`
	Transcript   []TranscriptSegment `json:"transcript"`
	Speakers     []string            `json:"speakers"`
	ErrorMessage string              `json:"errorMessage,omitempty"`
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
	ID                   string
	Status               string
	Filename             string
	CreatedAt            string
	ETASeconds           float64
	AudioTrackCount      *int
	AudioTracksCompleted *int
	AudioTrackNames      []string
	Raw                  map[string]any
}

// IsTerminal reports whether the job is done (VISION_COMPLETED, VIDEO_COMPLETED_NO_SCENES, or any FAILED status).
func (j *Job) IsTerminal() bool {
	return j.IsComplete() || j.IsFailed()
}

// IsComplete reports whether the job finished successfully (VISION_COMPLETED or VIDEO_COMPLETED_NO_SCENES).
func (j *Job) IsComplete() bool {
	return j.Status == "VISION_COMPLETED" || j.Status == "VIDEO_COMPLETED_NO_SCENES"
}

// IsFailed reports whether the job has failed (any status containing "FAILED").
func (j *Job) IsFailed() bool {
	return strings.Contains(j.Status, "FAILED")
}

// Result parses processedData from a completed job.
// Returns nil, false if the job isn't complete or has no processed data.
func (j *Job) Result() (*ProcessingResult, bool) {
	if !j.IsComplete() {
		return nil, false
	}
	if _, ok := j.Raw["processedData"]; !ok {
		return nil, false
	}
	return parseResult(j.Raw), true
}

// Quota holds the account's plan, included hours, credit balance, and reset date.
type Quota struct {
	Plan                string  `json:"currentPlan"`
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
	PollInterval   time.Duration
	Timeout        time.Duration
	OnProgress     func(*Job)
	CallbackURL    string
	ProcessingMode string // "all", "transcript", "vision"
	IdempotencyKey string
	AudioTracks    []AudioTrack
}

// UploadOptions overrides the filename derived from the file path.
type UploadOptions struct {
	Filename       string
	CallbackURL    string
	ProcessingMode string
	IdempotencyKey string
	AudioTracks    []AudioTrack
}

// ListJobsOptions filters and paginates ListJobs.
type ListJobsOptions struct {
	Limit  int
	Cursor string
	Status string
}

// BatchClip is a single video clip in a batch request.
type BatchClip struct {
	SourceURL     string `json:"sourceUrl"`
	FileName      string `json:"fileName,omitempty"`
	DownloadToken string `json:"downloadToken,omitempty"`
	Provider      string `json:"provider,omitempty"`
}

// BatchResult is returned by CreateBatch.
type BatchResult struct {
	BatchID string     `json:"batchId"`
	Mode    string     `json:"mode"`
	Jobs    []BatchJob `json:"jobs"`
}

// BatchJob is a single job entry in a BatchResult.
type BatchJob struct {
	JobID  string `json:"jobId"`
	Status string `json:"status"`
}

// BatchOptions configures CreateBatch and ProcessBatch.
type BatchOptions struct {
	Clips          []BatchClip
	Mode           string // "independent" or "continuous"
	ProcessingMode string
	CallbackURL    string
	PollInterval   time.Duration
	Timeout        time.Duration
	OnProgress     func([]Job)
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

type batchAPIResponse struct {
	BatchID string     `json:"batchId"`
	Mode    string     `json:"mode"`
	Jobs    []BatchJob `json:"jobs"`
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
	if v, ok := data["audioTrackCount"].(float64); ok {
		n := int(v)
		j.AudioTrackCount = &n
	}
	if v, ok := data["audioTracksCompleted"].(float64); ok {
		n := int(v)
		j.AudioTracksCompleted = &n
	}
	if names, ok := data["audioTrackNames"].([]any); ok {
		for _, name := range names {
			if s, ok := name.(string); ok {
				j.AudioTrackNames = append(j.AudioTrackNames, s)
			}
		}
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
