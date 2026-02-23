# frozen_string_literal: true

require "net/http"
require "json"
require "uri"

module FrameQuery
  # Synchronous client for the FrameQuery video processing API.
  #
  # @example
  #   client = FrameQuery::Client.new(api_key: "fq_...")
  #   result = client.process("video.mp4")
  #   puts result.scenes.map(&:description)
  class Client
    DEFAULT_BASE_URL = "https://api.framequery.com/v1/api"
    DEFAULT_POLL_INTERVAL = 5
    DEFAULT_TIMEOUT = 86_400
    DEFAULT_MAX_RETRIES = 2
    DEFAULT_HTTP_TIMEOUT = 300
    VERSION = "0.1.0"

    # @param api_key [String, nil] API key (falls back to FRAMEQUERY_API_KEY env var)
    # @param base_url [String] API base URL
    # @param timeout [Integer] HTTP timeout in seconds
    # @param max_retries [Integer] max retries on transient errors
    def initialize(api_key: nil, base_url: DEFAULT_BASE_URL, timeout: DEFAULT_HTTP_TIMEOUT, max_retries: DEFAULT_MAX_RETRIES)
      @api_key = api_key || ENV.fetch("FRAMEQUERY_API_KEY") {
        raise ArgumentError, "api_key is required. Pass it explicitly or set FRAMEQUERY_API_KEY."
      }
      @base_url = base_url.chomp("/")
      @timeout = timeout
      @max_retries = max_retries
    end

    # Upload a video file and wait for processing to complete.
    #
    # @param file_path [String] path to the local video file
    # @param filename [String, nil] object name override
    # @param poll_interval [Integer] seconds between status polls
    # @param timeout [Integer] maximum seconds to wait
    # @yield [Job] optional progress callback invoked on each poll
    # @return [ProcessingResult]
    def process(file_path, filename: nil, poll_interval: DEFAULT_POLL_INTERVAL, timeout: DEFAULT_TIMEOUT, &on_progress)
      job = upload(file_path, filename: filename)
      poll(job.id, poll_interval, timeout, &on_progress)
    end

    # Submit a URL for processing and wait for completion.
    #
    # @param url [String] public HTTP(S) URL of the video
    # @param filename [String, nil] optional filename hint
    # @param poll_interval [Integer] seconds between polls
    # @param timeout [Integer] max seconds to wait
    # @yield [Job] optional progress callback
    # @return [ProcessingResult]
    def process_url(url, filename: nil, poll_interval: DEFAULT_POLL_INTERVAL, timeout: DEFAULT_TIMEOUT, &on_progress)
      body = { url: url }
      body[:fileName] = filename if filename
      data = request(:post, "/jobs/from-url", body: body)
      job = Parsers.parse_job(data)
      poll(job.id, poll_interval, timeout, &on_progress)
    end

    # Upload a video and return the Job immediately (does not wait).
    #
    # @param file_path [String] path to the local video file
    # @param filename [String, nil] object name override
    # @return [Job]
    def upload(file_path, filename: nil)
      raise Errno::ENOENT, file_path unless File.file?(file_path)

      name = filename || File.basename(file_path)
      data = request(:post, "/jobs", body: { fileName: name })
      upload_url = data["uploadUrl"]

      # PUT file to signed URL
      uri = URI.parse(upload_url)
      http = Net::HTTP.new(uri.host, uri.port)
      http.use_ssl = uri.scheme == "https"
      http.open_timeout = @timeout
      http.read_timeout = @timeout

      file_data = File.binread(file_path)
      put_req = Net::HTTP::Put.new(uri)
      put_req["Content-Type"] = "application/octet-stream"
      put_req.body = file_data

      resp = http.request(put_req)
      unless resp.is_a?(Net::HTTPSuccess)
        raise Error, "Upload to signed URL failed with status #{resp.code}"
      end

      Parsers.parse_job(data)
    end

    # Fetch the current state of a job.
    #
    # @param job_id [String]
    # @return [Job]
    def get_job(job_id)
      data = request(:get, "/jobs/#{URI.encode_www_form_component(job_id)}")
      Parsers.parse_job(data)
    end

    # List jobs with optional filtering and pagination.
    #
    # @param limit [Integer]
    # @param cursor [String, nil]
    # @param status [String, nil]
    # @return [JobPage]
    def list_jobs(limit: 20, cursor: nil, status: nil)
      params = { limit: limit }
      params[:cursor] = cursor if cursor
      params[:status] = status if status
      raw = request_raw(:get, "/jobs", params: params)
      items = raw["data"] || []
      jobs = items.map { |j| Parsers.parse_job(j) }
      JobPage.new(jobs: jobs, next_cursor: raw["nextCursor"])
    end

    # Get the current account quota.
    #
    # @return [Quota]
    def get_quota
      data = request(:get, "/quota")
      Parsers.parse_quota(data)
    end

    private

    # Make a request, unwrap the "data" envelope.
    def request(method, path, body: nil, params: nil)
      raw = request_raw(method, path, body: body, params: params)
      raw.key?("data") ? raw["data"] : raw
    end

    # Make a request, return raw JSON hash.
    def request_raw(method, path, body: nil, params: nil)
      uri = build_uri(path, params)
      last_error = nil

      (@max_retries + 1).times do |attempt|
        begin
          http = Net::HTTP.new(uri.host, uri.port)
          http.use_ssl = uri.scheme == "https"
          http.open_timeout = @timeout
          http.read_timeout = @timeout

          req = build_request(method, uri, body)
          resp = http.request(req)

          # Retry on 5xx / 429
          if resp.code.to_i >= 500 || resp.code.to_i == 429
            if attempt < @max_retries
              delay = backoff_delay(attempt, resp)
              sleep(delay)
              next
            end
          end

          handle_error_response(resp) unless resp.is_a?(Net::HTTPSuccess)
          return JSON.parse(resp.body || "{}")
        rescue Net::OpenTimeout, Net::ReadTimeout, Errno::ECONNRESET, Errno::ECONNREFUSED => e
          last_error = e
          if attempt < @max_retries
            sleep(backoff_delay(attempt))
            next
          end
          raise Error, "Request failed after retries: #{e.message}"
        end
      end

      raise Error, "Request failed: #{last_error&.message}"
    end

    def build_uri(path, params)
      url = "#{@base_url}#{path}"
      if params && !params.empty?
        query = URI.encode_www_form(params)
        url += "?#{query}"
      end
      URI.parse(url)
    end

    def build_request(method, uri, body)
      req = case method
            when :get    then Net::HTTP::Get.new(uri)
            when :post   then Net::HTTP::Post.new(uri)
            when :put    then Net::HTTP::Put.new(uri)
            when :delete then Net::HTTP::Delete.new(uri)
            else raise ArgumentError, "unsupported method: #{method}"
            end

      req["Authorization"] = "Bearer #{@api_key}"
      req["User-Agent"] = "framequery-ruby/#{VERSION}"

      if body
        req["Content-Type"] = "application/json"
        req.body = JSON.generate(body)
      end

      req
    end

    def handle_error_response(resp)
      status = resp.code.to_i
      message = "API error #{status}"
      body = nil

      begin
        body = JSON.parse(resp.body || "{}")
        msg = body["error"] || body["message"]
        message = msg.to_s if msg
      rescue JSON::ParserError
        message = resp.body.to_s if resp.body && !resp.body.empty?
      end

      case status
      when 401 then raise AuthenticationError, message
      when 403 then raise PermissionDeniedError, message
      when 404 then raise NotFoundError, message
      when 429
        retry_after = resp["Retry-After"]&.to_f
        raise RateLimitError.new(message, retry_after: retry_after)
      else
        raise APIError.new(message, status_code: status, body: body)
      end
    end

    def poll(job_id, poll_interval, timeout, &on_progress)
      deadline = Time.now + timeout
      interval = poll_interval

      loop do
        job = get_job(job_id)
        on_progress&.call(job)

        if job.failed?
          error_msg = job.raw["errorMessage"].to_s
          raise JobFailedError.new(job_id, error_msg)
        end

        return Parsers.parse_result(job.raw) if job.complete?

        if Time.now > deadline
          raise FrameQuery::TimeoutError, "Timed out after #{timeout}s waiting for job #{job_id}"
        end

        # Adaptive polling
        if job.eta_seconds && job.eta_seconds > 60
          interval = [job.eta_seconds / 3.0, 30.0].min
        else
          interval = poll_interval
        end

        sleep(interval)
      end
    end

    def backoff_delay(attempt, resp = nil)
      if resp
        ra = resp["Retry-After"]
        return ra.to_f if ra
      end
      [0.5 * (2**attempt), 30.0].min
    end
  end
end
