# frozen_string_literal: true

module FrameQuery
  class Error < StandardError; end

  # 401
  class AuthenticationError < Error; end

  # 403
  class PermissionDeniedError < Error; end

  # 404
  class NotFoundError < Error; end

  # 429 — check retry_after for the server-suggested wait (may be nil)
  class RateLimitError < Error
    attr_reader :retry_after

    def initialize(message = "Rate limit exceeded", retry_after: nil)
      super(message)
      @retry_after = retry_after
    end
  end

  # Any other non-2xx response. status_code and body give you the raw details.
  class APIError < Error
    attr_reader :status_code, :body

    def initialize(message, status_code:, body: nil)
      super(message)
      @status_code = status_code
      @body = body
    end
  end

  # Job reached FAILED status during polling.
  class JobFailedError < Error
    attr_reader :job_id

    def initialize(job_id, message = "")
      msg = "Job #{job_id} failed"
      msg += ": #{message}" unless message.empty?
      super(msg)
      @job_id = job_id
    end
  end

  # Polling exceeded the timeout.
  class TimeoutError < Error; end
end
