# frozen_string_literal: true

module FrameQuery
  # Base error class for all FrameQuery SDK errors.
  class Error < StandardError; end

  # Raised when the API key is invalid or missing (HTTP 401).
  class AuthenticationError < Error; end

  # Raised when the API key lacks required scopes (HTTP 403).
  class PermissionDeniedError < Error; end

  # Raised when the requested resource does not exist (HTTP 404).
  class NotFoundError < Error; end

  # Raised when the API rate limit is exceeded (HTTP 429).
  class RateLimitError < Error
    # @return [Float, nil] seconds to wait before retrying
    attr_reader :retry_after

    def initialize(message = "Rate limit exceeded", retry_after: nil)
      super(message)
      @retry_after = retry_after
    end
  end

  # Raised for unexpected HTTP errors from the API.
  class APIError < Error
    # @return [Integer] HTTP status code
    attr_reader :status_code
    # @return [Hash, nil] parsed response body
    attr_reader :body

    def initialize(message, status_code:, body: nil)
      super(message)
      @status_code = status_code
      @body = body
    end
  end

  # Raised when a polled job reaches FAILED status.
  class JobFailedError < Error
    # @return [String] the failed job's ID
    attr_reader :job_id

    def initialize(job_id, message = "")
      msg = "Job #{job_id} failed"
      msg += ": #{message}" unless message.empty?
      super(msg)
      @job_id = job_id
    end
  end

  # Raised when polling times out.
  class TimeoutError < Error; end
end
