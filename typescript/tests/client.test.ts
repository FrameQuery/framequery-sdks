import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { FrameQuery } from "../src/client";
import {
  AuthenticationError,
  FrameQueryError,
  NotFoundError,
} from "../src/errors";
import {
  parseAudioTrackTranscript,
  parseJob,
  parseQuota,
} from "../src/models";

const completedJobPayload = {
  jobId: "job_123",
  status: "VISION_COMPLETED",
  originalFilename: "demo.mp4",
  createdAt: "2026-01-01T00:00:00Z",
  processedData: {
    length: 12.5,
    scenes: [{ description: "intro", endTs: 3.2, objects: ["desk", "mug"] }],
    transcript: [{ StartTime: 0, EndTime: 2.5, Text: "hello" }],
  },
};

describe("parseJob", () => {
  it("parses a pending job", () => {
    const job = parseJob({
      jobId: "job_1",
      status: "PROCESSING",
      originalFilename: "clip.mp4",
      estimatedCompletionTimeSeconds: 42,
    });
    expect(job.id).toBe("job_1");
    expect(job.etaSeconds).toBe(42);
    expect(job.isTerminal).toBe(false);
    expect(job.isComplete).toBe(false);
    expect(job.isFailed).toBe(false);
    expect(job.result).toBeNull();
  });

  it("parses a completed job including its result", () => {
    const job = parseJob(completedJobPayload);
    expect(job.isComplete).toBe(true);
    expect(job.isTerminal).toBe(true);
    expect(job.result).not.toBeNull();
    expect(job.result?.duration).toBe(12.5);
    expect(job.result?.scenes).toEqual([
      { description: "intro", endTime: 3.2, objects: ["desk", "mug"] },
    ]);
    expect(job.result?.transcript).toEqual([
      { startTime: 0, endTime: 2.5, text: "hello" },
    ]);
  });

  it("parses a failed job", () => {
    const job = parseJob({ jobId: "job_2", status: "VISION_FAILED" });
    expect(job.isFailed).toBe(true);
    expect(job.isTerminal).toBe(true);
    expect(job.isComplete).toBe(false);
  });

  it("parses multi-track audio fields", () => {
    const job = parseJob({
      jobId: "job_3",
      status: "PROCESSING",
      audioTrackCount: 2,
      audioTracksCompleted: 1,
      audioTrackNames: ["Host", "Guest"],
    });
    expect(job.audioTrackCount).toBe(2);
    expect(job.audioTracksCompleted).toBe(1);
    expect(job.audioTrackNames).toEqual(["Host", "Guest"]);
  });
});

describe("parseAudioTrackTranscript", () => {
  it("parses a track and accepts both segment key casings", () => {
    const track = parseAudioTrackTranscript({
      trackIndex: 1,
      trackName: "Guest",
      language: "en",
      status: "COMPLETED",
      speakers: ["A"],
      transcript: [
        { startTime: 0, endTime: 1, text: "hi" },
        { StartTime: 1, EndTime: 2, Text: "there" },
      ],
    });
    expect(track.trackIndex).toBe(1);
    expect(track.trackName).toBe("Guest");
    expect(track.speakers).toEqual(["A"]);
    expect(track.transcript).toEqual([
      { startTime: 0, endTime: 1, text: "hi" },
      { startTime: 1, endTime: 2, text: "there" },
    ]);
    expect(track.errorMessage).toBeUndefined();
  });
});

describe("parseQuota", () => {
  it("maps the quota payload", () => {
    const quota = parseQuota({
      currentPlan: "pro",
      includedHours: 10,
      creditsBalanceHours: 4.5,
      resetDate: "2026-02-01",
    });
    expect(quota).toEqual({
      plan: "pro",
      includedHours: 10,
      creditsBalanceHours: 4.5,
      resetDate: "2026-02-01",
    });
  });
});

describe("FrameQuery client", () => {
  let savedKey: string | undefined;

  beforeEach(() => {
    savedKey = process.env.FRAMEQUERY_API_KEY;
    delete process.env.FRAMEQUERY_API_KEY;
  });

  afterEach(() => {
    if (savedKey === undefined) {
      delete process.env.FRAMEQUERY_API_KEY;
    } else {
      process.env.FRAMEQUERY_API_KEY = savedKey;
    }
  });

  function jsonResponse(body: unknown, status = 200): Response {
    return new Response(JSON.stringify(body), {
      status,
      headers: { "Content-Type": "application/json" },
    });
  }

  function clientWith(handler: (url: string) => Response): FrameQuery {
    const fetchStub = (async (input: RequestInfo | URL) =>
      handler(String(input))) as typeof globalThis.fetch;
    return new FrameQuery({ apiKey: "test-key", maxRetries: 0, fetch: fetchStub });
  }

  it("throws without an API key", () => {
    expect(() => new FrameQuery()).toThrow(FrameQueryError);
  });

  it("fetches and parses a job", async () => {
    const client = clientWith((url) => {
      expect(url).toBe("https://api.framequery.com/v1/api/jobs/job_123");
      return jsonResponse({ data: completedJobPayload });
    });
    const job = await client.getJob("job_123");
    expect(job.id).toBe("job_123");
    expect(job.isComplete).toBe(true);
    expect(job.result?.filename).toBe("demo.mp4");
  });

  it("fetches audio tracks for a job", async () => {
    const client = clientWith(() =>
      jsonResponse({
        tracks: [
          { trackIndex: 0, trackName: "Host", language: "en", status: "COMPLETED", transcript: [] },
        ],
      }),
    );
    const tracks = await client.getAudioTracks("job_123");
    expect(tracks).toHaveLength(1);
    expect(tracks[0].trackName).toBe("Host");
  });

  it("maps 401 to AuthenticationError", async () => {
    const client = clientWith(() => jsonResponse({ error: "bad key" }, 401));
    await expect(client.getJob("job_x")).rejects.toBeInstanceOf(AuthenticationError);
  });

  it("maps 404 to NotFoundError", async () => {
    const client = clientWith(() => jsonResponse({ error: "no such job" }, 404));
    await expect(client.getJob("job_x")).rejects.toBeInstanceOf(NotFoundError);
  });
});
