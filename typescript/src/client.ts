import {
  APIError,
  AuthenticationError,
  FrameQueryError,
  JobFailedError,
  NotFoundError,
  PermissionDeniedError,
  RateLimitError,
} from "./errors.js";
import type {
  FrameQueryOptions,
  Job,
  JobPage,
  ListJobsOptions,
  ProcessingResult,
  ProcessOptions,
  Quota,
  UploadOptions,
} from "./models.js";
import { parseJob, parseQuota, parseResult } from "./models.js";

const DEFAULT_BASE_URL = "https://api.framequery.com/v1/api";
const DEFAULT_POLL_INTERVAL = 5_000;
const DEFAULT_TIMEOUT = 86_400_000; // 24h
const DEFAULT_MAX_RETRIES = 2;
const VERSION = "0.1.0";

/** Main client. Talks to the FrameQuery REST API. */
export class FrameQuery {
  private readonly baseUrl: string;
  private readonly apiKey: string;
  private readonly maxRetries: number;
  private readonly fetchImpl: typeof globalThis.fetch;
  private readonly httpTimeout: number;

  constructor(options?: FrameQueryOptions) {
    const key = options?.apiKey ?? getEnvKey();
    if (!key) {
      throw new FrameQueryError(
        "apiKey is required. Pass it in options or set FRAMEQUERY_API_KEY.",
      );
    }
    this.apiKey = key;
    this.baseUrl = (options?.baseUrl ?? DEFAULT_BASE_URL).replace(/\/+$/, "");
    this.maxRetries = options?.maxRetries ?? DEFAULT_MAX_RETRIES;
    this.httpTimeout = options?.timeout ?? 300_000;

    const f = options?.fetch ?? globalThis.fetch;
    if (!f) {
      throw new FrameQueryError(
        "No fetch implementation found. Use Node 18+ or pass a fetch implementation.",
      );
    }
    this.fetchImpl = f.bind(globalThis);
  }

  /**
   * Upload + poll until done. Pass a file path (Node) or Blob/ArrayBuffer (browser).
   * Times out after 24h by default. Override with `options.timeout`.
   */
  async process(
    file: string | Blob | ArrayBuffer | Uint8Array,
    options?: ProcessOptions,
  ): Promise<ProcessingResult> {
    const job = await this.upload(file, options);
    return this.poll(
      job.id,
      options?.pollInterval ?? DEFAULT_POLL_INTERVAL,
      options?.timeout ?? DEFAULT_TIMEOUT,
      options?.onProgress,
      options?.signal,
    );
  }

  /** Same as process() but takes a public URL instead of a file. */
  async processUrl(
    url: string,
    options?: ProcessOptions,
  ): Promise<ProcessingResult> {
    const body: Record<string, string> = { url };
    if (options?.filename) body.fileName = options.filename;

    const data = await this.request<Record<string, unknown>>("POST", "/jobs/from-url", {
      body: JSON.stringify(body),
    });
    const job = parseJob(data);
    return this.poll(
      job.id,
      options?.pollInterval ?? DEFAULT_POLL_INTERVAL,
      options?.timeout ?? DEFAULT_TIMEOUT,
      options?.onProgress,
      options?.signal,
    );
  }

  /** Upload only -- returns the Job without polling. */
  async upload(
    file: string | Blob | ArrayBuffer | Uint8Array,
    options?: UploadOptions,
  ): Promise<Job> {
    let fileContent: Blob | ArrayBuffer | Uint8Array | Buffer;
    let filename: string;

    if (typeof file === "string") {
      // file path -- read from disk (Node only)
      const fs = await import("node:fs/promises");
      const path = await import("node:path");
      fileContent = await fs.readFile(file);
      filename = options?.filename ?? path.basename(file);
    } else {
      fileContent = file;
      filename = options?.filename ?? "video.mp4";
    }

    const data = await this.request<Record<string, unknown>>("POST", "/jobs", {
      body: JSON.stringify({ fileName: filename }),
    });

    const uploadUrl = String(data.uploadUrl);

    const uploadResp = await this.fetchImpl(uploadUrl, {
      method: "PUT",
      body: fileContent,
      headers: { "Content-Type": "application/octet-stream" },
      signal: options?.signal,
    });

    if (!uploadResp.ok) {
      const text = await uploadResp.text().catch(() => "");
      throw new FrameQueryError(
        `Upload failed with status ${uploadResp.status}${text ? `: ${text}` : ""}`,
      );
    }

    return parseJob(data);
  }

  /** Poll a single job by ID. */
  async getJob(jobId: string): Promise<Job> {
    const data = await this.request<Record<string, unknown>>("GET", `/jobs/${encodeURIComponent(jobId)}`);
    return parseJob(data);
  }

  /** Paginated job list. Cursor-based. */
  async listJobs(options?: ListJobsOptions): Promise<JobPage> {
    const params = new URLSearchParams();
    if (options?.limit) params.set("limit", String(options.limit));
    if (options?.cursor) params.set("cursor", options.cursor);
    if (options?.status) params.set("status", options.status);

    const qs = params.toString();
    const path = qs ? `/jobs?${qs}` : "/jobs";
    const raw = await this.requestRaw("GET", path);

    const items = (raw.data as Record<string, unknown>[]) ?? [];
    const jobs = items.map(parseJob);
    const nextCursor = raw.nextCursor ? String(raw.nextCursor) : null;

    return { jobs, nextCursor, hasMore: nextCursor !== null };
  }

  /** Returns plan info and remaining credit hours. */
  async getQuota(): Promise<Quota> {
    const data = await this.request<Record<string, unknown>>("GET", "/quota");
    return parseQuota(data);
  }

  // -- internals --

  private async request<T>(
    method: string,
    path: string,
    init?: { body?: string },
  ): Promise<T> {
    const resp = await this.doRequest(method, path, init);
    return this.handleResponse<T>(resp);
  }

  private async requestRaw(
    method: string,
    path: string,
  ): Promise<Record<string, unknown>> {
    const resp = await this.doRequest(method, path);
    if (!resp.ok) {
      await this.handleErrorResponse(resp);
    }
    return (await resp.json()) as Record<string, unknown>;
  }

  private async doRequest(
    method: string,
    path: string,
    init?: { body?: string },
  ): Promise<Response> {
    const url = `${this.baseUrl}${path}`;
    let lastError: Error | undefined;

    for (let attempt = 0; attempt <= this.maxRetries; attempt++) {
      try {
        const headers: Record<string, string> = {
          Authorization: `Bearer ${this.apiKey}`,
          "User-Agent": `framequery-node/${VERSION}`,
        };
        if (init?.body) {
          headers["Content-Type"] = "application/json";
        }

        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), this.httpTimeout);

        const resp = await this.fetchImpl(url, {
          method,
          headers,
          body: init?.body,
          signal: controller.signal,
        });

        clearTimeout(timeoutId);

        if (resp.status < 500 && resp.status !== 429) {
          return resp;
        }

        if (attempt < this.maxRetries) {
          const delay = backoffDelay(attempt, resp);
          await sleep(delay);
          continue;
        }
        return resp;
      } catch (err) {
        lastError = err instanceof Error ? err : new Error(String(err));
        if (attempt < this.maxRetries) {
          await sleep(backoffDelay(attempt));
          continue;
        }
      }
    }

    throw new FrameQueryError(
      `Request failed after retries: ${lastError?.message ?? "unknown"}`,
    );
  }

  private async handleResponse<T>(resp: Response): Promise<T> {
    if (resp.ok) {
      const json = (await resp.json()) as Record<string, unknown>;
      if ("data" in json) return json.data as T;
      return json as T;
    }
    await this.handleErrorResponse(resp);
    throw new FrameQueryError("Unreachable"); // handleErrorResponse always throws
  }

  private async handleErrorResponse(resp: Response): Promise<never> {
    let message = `API error ${resp.status}`;
    let body: Record<string, unknown> | undefined;

    try {
      body = (await resp.json()) as Record<string, unknown>;
      const msg = body?.error ?? body?.message;
      if (msg) message = String(msg);
    } catch {
      const text = await resp.text().catch(() => "");
      if (text) message = text;
    }

    if (resp.status === 401) throw new AuthenticationError(message);
    if (resp.status === 403) throw new PermissionDeniedError(message);
    if (resp.status === 404) throw new NotFoundError(message);
    if (resp.status === 429) {
      const ra = resp.headers.get("Retry-After");
      throw new RateLimitError(message, ra ? parseFloat(ra) : undefined);
    }
    throw new APIError(message, resp.status, body);
  }

  private async poll(
    jobId: string,
    pollIntervalMs: number,
    timeoutMs: number,
    onProgress?: (job: Job) => void,
    signal?: AbortSignal,
  ): Promise<ProcessingResult> {
    const deadline = Date.now() + timeoutMs;
    let interval = pollIntervalMs;

    while (true) {
      if (signal?.aborted) {
        throw new FrameQueryError("Aborted");
      }

      const job = await this.getJob(jobId);

      if (onProgress) onProgress(job);

      if (job.isFailed) {
        const errorMsg = String((job.raw as Record<string, unknown>).errorMessage ?? "");
        throw new JobFailedError(jobId, errorMsg);
      }

      if (job.isComplete) {
        return parseResult(job.raw);
      }

      if (Date.now() > deadline) {
        throw new FrameQueryError(
          `Timed out after ${timeoutMs}ms waiting for job ${jobId}`,
        );
      }

      // Back off when ETA is long so we're not hammering the API
      if (job.etaSeconds && job.etaSeconds > 60) {
        interval = Math.min(job.etaSeconds / 3 * 1000, 30_000);
      } else {
        interval = pollIntervalMs;
      }

      await sleep(interval);
    }
  }
}

function getEnvKey(): string | undefined {
  if (typeof process !== "undefined" && process.env) {
    return process.env.FRAMEQUERY_API_KEY;
  }
  return undefined;
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function backoffDelay(attempt: number, resp?: Response): number {
  if (resp) {
    const ra = resp.headers.get("Retry-After");
    if (ra) {
      const val = parseFloat(ra);
      if (!isNaN(val)) return val * 1000;
    }
  }
  return Math.min(500 * 2 ** attempt, 30_000);
}

export default FrameQuery;
