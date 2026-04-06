export interface Scene {
  description: string;
  endTime: number;
  objects: string[];
}

export interface TranscriptSegment {
  startTime: number;
  endTime: number;
  text: string;
}

export interface ProcessingResult {
  jobId: string;
  status: string;
  filename: string;
  duration: number;
  scenes: Scene[];
  transcript: TranscriptSegment[];
  createdAt: string;
  raw: Record<string, unknown>;
}

export interface Job {
  id: string;
  status: string;
  filename: string;
  createdAt: string;
  etaSeconds?: number;
  raw: Record<string, unknown>;
  isTerminal: boolean;
  isComplete: boolean;
  isFailed: boolean;
  /** Parsed processing result, available when the job is complete. */
  result: ProcessingResult | null;
  /** Multi-track audio: total tracks expected. */
  audioTrackCount?: number;
  /** Multi-track audio: tracks with STT completed. */
  audioTracksCompleted?: number;
  /** Multi-track audio: track labels (from iXML or user-provided). */
  audioTrackNames?: string[];
}

export interface Quota {
  plan: string;
  includedHours: number;
  creditsBalanceHours: number;
  resetDate: string | null;
}

export interface JobPage {
  jobs: Job[];
  nextCursor: string | null;
  hasMore: boolean;
}

export interface FrameQueryOptions {
  apiKey?: string;
  baseUrl?: string;
  timeout?: number;
  maxRetries?: number;
  fetch?: typeof globalThis.fetch;
}

export interface ProcessOptions {
  filename?: string;
  pollInterval?: number;
  timeout?: number;
  onProgress?: (job: Job) => void;
  signal?: AbortSignal;
  /** HTTPS webhook URL called when the job completes or fails. */
  callbackUrl?: string;
  /** Which pipeline stages to run: "all" (default), "transcript", or "vision". */
  processingMode?: "all" | "transcript" | "vision";
  /** Client-generated key to prevent duplicate job creation (24h TTL). */
  idempotencyKey?: string;
  /** Multiple audio tracks with per-track sync params (max 16). Mutually exclusive with single audio file. */
  audioTracks?: AudioTrack[];
}

export interface UploadOptions {
  filename?: string;
  signal?: AbortSignal;
  /** HTTPS webhook URL called when the job completes or fails. */
  callbackUrl?: string;
  /** Which pipeline stages to run: "all" (default), "transcript", or "vision". */
  processingMode?: "all" | "transcript" | "vision";
  /** Client-generated key to prevent duplicate job creation (24h TTL). */
  idempotencyKey?: string;
  /** Multiple audio tracks with per-track sync params (max 16). Mutually exclusive with single audio file. */
  audioTracks?: AudioTrack[];
}

export interface ListJobsOptions {
  limit?: number;
  cursor?: string;
  status?: string;
}

export interface AudioTrack {
  /** Filename for presigned upload flow. */
  fileName?: string;
  /** Direct URL for from-url flow. */
  url?: string;
  /** Auth token for URL downloads. */
  downloadToken?: string;
  /** "auto" (default), "timecode", or "offset". */
  syncMode?: string;
  /** Manual offset in ms when syncMode="offset". */
  offsetMs?: number;
  /** User-provided label (e.g., "Host", "Guest"). */
  label?: string;
  /** Split polyphonic file into per-channel transcripts. */
  perChannelTranscription?: boolean;
  /** Select specific channels from polyphonic file (0-indexed). */
  channels?: number[];
}

export interface AudioTrackTranscript {
  trackIndex: number;
  trackName: string;
  language: string;
  status: string;
  speakers?: string[];
  transcript: TranscriptSegment[];
  errorMessage?: string;
}

export interface BatchClip {
  /** Public URL to the video file. */
  sourceUrl: string;
  /** Override filename (defaults to URL filename). */
  fileName?: string;
  /** Bearer token for authenticated downloads (e.g. Google Drive OAuth). */
  downloadToken?: string;
  /** Cloud provider hint: "gdrive", "dropbox", or omit for plain URL. */
  provider?: string;
}

export interface BatchOptions {
  /** Array of video clips to process. */
  clips: BatchClip[];
  /** "independent" = each clip is a separate job; "continuous" = clips concatenated into one job. */
  mode: "independent" | "continuous";
  /** Which pipeline stages to run: "all" (default), "transcript", or "vision". */
  processingMode?: "all" | "transcript" | "vision";
  /** HTTPS webhook URL called when jobs complete or fail. */
  callbackUrl?: string;
  /** Polling interval in ms for processBatch (default 5000). */
  pollInterval?: number;
  /** Polling timeout in ms for processBatch (default 24h). */
  timeout?: number;
  /** Called on each poll tick with current job states. */
  onProgress?: (jobs: Job[]) => void;
  /** AbortSignal to cancel polling. */
  signal?: AbortSignal;
}

export interface BatchResult {
  batchId: string;
  mode: string;
  jobs: { jobId: string; status: string }[];
}

// -- parsers (map raw API response -> typed objects) --

export function parseJob(data: Record<string, unknown>): Job {
  const status = String(data.status ?? "");
  const isComplete = status === "VISION_COMPLETED" || status === "VIDEO_COMPLETED_NO_SCENES";
  const isFailed = status.includes("FAILED");
  const hasResult = isComplete && data.processedData != null;
  return {
    id: String(data.jobId ?? ""),
    status,
    filename: String(data.originalFilename ?? ""),
    createdAt: String(data.createdAt ?? ""),
    etaSeconds: typeof data.estimatedCompletionTimeSeconds === "number"
      ? data.estimatedCompletionTimeSeconds
      : undefined,
    raw: data,
    isTerminal: isComplete || isFailed,
    isComplete,
    isFailed,
    result: hasResult ? parseResult(data) : null,
    audioTrackCount: typeof data.audioTrackCount === "number" ? data.audioTrackCount : undefined,
    audioTracksCompleted: typeof data.audioTracksCompleted === "number" ? data.audioTracksCompleted : undefined,
    audioTrackNames: Array.isArray(data.audioTrackNames) ? data.audioTrackNames.map(String) : undefined,
  };
}

export function parseResult(data: Record<string, unknown>): ProcessingResult {
  const processed = (data.processedData as Record<string, unknown>) ?? {};
  const rawScenes = (processed.scenes as Record<string, unknown>[]) ?? [];
  const rawTranscript = (processed.transcript as Record<string, unknown>[]) ?? [];

  return {
    jobId: String(data.jobId ?? ""),
    status: String(data.status ?? ""),
    filename: String(data.originalFilename ?? ""),
    duration: Number(processed.length ?? 0),
    scenes: rawScenes.map((s) => ({
      description: String(s.description ?? ""),
      endTime: Number(s.endTs ?? 0),
      objects: Array.isArray(s.objects) ? s.objects.map(String) : [],
    })),
    transcript: rawTranscript.map((t) => ({
      startTime: Number(t.StartTime ?? 0),
      endTime: Number(t.EndTime ?? 0),
      text: String(t.Text ?? ""),
    })),
    createdAt: String(data.createdAt ?? ""),
    raw: data,
  };
}

export function parseAudioTrackTranscript(data: Record<string, unknown>): AudioTrackTranscript {
  const rawTranscript = (data.transcript as Record<string, unknown>[]) ?? [];
  return {
    trackIndex: Number(data.trackIndex ?? 0),
    trackName: String(data.trackName ?? ""),
    language: String(data.language ?? ""),
    status: String(data.status ?? ""),
    speakers: Array.isArray(data.speakers) ? data.speakers.map(String) : undefined,
    transcript: rawTranscript.map((t) => ({
      startTime: Number(t.startTime ?? t.StartTime ?? 0),
      endTime: Number(t.endTime ?? t.EndTime ?? 0),
      text: String(t.text ?? t.Text ?? ""),
    })),
    errorMessage: data.errorMessage ? String(data.errorMessage) : undefined,
  };
}

export function parseQuota(data: Record<string, unknown>): Quota {
  return {
    plan: String(data.currentPlan ?? ""),
    includedHours: Number(data.includedHours ?? 0),
    creditsBalanceHours: Number(data.creditsBalanceHours ?? 0),
    resetDate: data.resetDate ? String(data.resetDate) : null,
  };
}
