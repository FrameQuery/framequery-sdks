export class FrameQueryError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "FrameQueryError";
  }
}

export class AuthenticationError extends FrameQueryError {
  constructor(message: string = "Authentication failed") {
    super(message);
    this.name = "AuthenticationError";
  }
}

export class PermissionDeniedError extends FrameQueryError {
  constructor(message: string = "Permission denied") {
    super(message);
    this.name = "PermissionDeniedError";
  }
}

export class NotFoundError extends FrameQueryError {
  constructor(message: string = "Resource not found") {
    super(message);
    this.name = "NotFoundError";
  }
}

export class RateLimitError extends FrameQueryError {
  retryAfter?: number;

  constructor(message: string = "Rate limit exceeded", retryAfter?: number) {
    super(message);
    this.name = "RateLimitError";
    this.retryAfter = retryAfter;
  }
}

export class APIError extends FrameQueryError {
  statusCode: number;
  body?: Record<string, unknown>;

  constructor(
    message: string,
    statusCode: number,
    body?: Record<string, unknown>,
  ) {
    super(message);
    this.name = "APIError";
    this.statusCode = statusCode;
    this.body = body;
  }
}

export class JobFailedError extends FrameQueryError {
  jobId: string;

  constructor(jobId: string, message: string = "") {
    const msg = message ? `Job ${jobId} failed: ${message}` : `Job ${jobId} failed`;
    super(msg);
    this.name = "JobFailedError";
    this.jobId = jobId;
  }
}
