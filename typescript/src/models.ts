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
}

export interface UploadOptions {
  filename?: string;
  signal?: AbortSignal;
}

export interface ListJobsOptions {
  limit?: number;
  cursor?: string;
  status?: string;
}

// ---- Internal parsers ----

export function parseJob(data: Record<string, unknown>): Job {
  const status = String(data.status ?? "");
  return {
    id: String(data.jobId ?? ""),
    status,
    filename: String(data.originalFilename ?? ""),
    createdAt: String(data.createdAt ?? ""),
    etaSeconds: typeof data.estimatedCompletionTimeSeconds === "number"
      ? data.estimatedCompletionTimeSeconds
      : undefined,
    raw: data,
    isTerminal: status === "COMPLETED" || status === "COMPLETED_NO_SCENES" || status === "FAILED",
    isComplete: status === "COMPLETED" || status === "COMPLETED_NO_SCENES",
    isFailed: status === "FAILED",
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

export function parseQuota(data: Record<string, unknown>): Quota {
  return {
    plan: String(data.plan ?? ""),
    includedHours: Number(data.includedHours ?? 0),
    creditsBalanceHours: Number(data.creditsBalanceHours ?? 0),
    resetDate: data.resetDate ? String(data.resetDate) : null,
  };
}
