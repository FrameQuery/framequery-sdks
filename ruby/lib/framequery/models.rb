# frozen_string_literal: true

module FrameQuery
  # A single detected scene in the video.
  class Scene
    # @return [String]
    attr_reader :description
    # @return [Float]
    attr_reader :end_time
    # @return [Array<String>]
    attr_reader :objects

    def initialize(description:, end_time:, objects: [])
      @description = description
      @end_time = end_time
      @objects = objects
    end
  end

  # A segment of the video transcript.
  class TranscriptSegment
    # @return [Float]
    attr_reader :start_time
    # @return [Float]
    attr_reader :end_time
    # @return [String]
    attr_reader :text

    def initialize(start_time:, end_time:, text:)
      @start_time = start_time
      @end_time = end_time
      @text = text
    end
  end

  # Complete result of a processed video job.
  class ProcessingResult
    attr_reader :job_id, :status, :filename, :duration, :scenes, :transcript, :created_at, :raw

    def initialize(job_id:, status:, filename:, duration:, scenes:, transcript:, created_at:, raw:)
      @job_id = job_id
      @status = status
      @filename = filename
      @duration = duration
      @scenes = scenes
      @transcript = transcript
      @created_at = created_at
      @raw = raw
    end
  end

  # A video processing job.
  class Job
    attr_reader :id, :status, :filename, :created_at, :eta_seconds, :raw

    def initialize(id:, status:, filename:, created_at:, eta_seconds: nil, raw: {})
      @id = id
      @status = status
      @filename = filename
      @created_at = created_at
      @eta_seconds = eta_seconds
      @raw = raw
    end

    # @return [Boolean] true if the job has reached a final state.
    def terminal?
      %w[COMPLETED COMPLETED_NO_SCENES FAILED].include?(@status)
    end

    # @return [Boolean] true if the job completed successfully.
    def complete?
      %w[COMPLETED COMPLETED_NO_SCENES].include?(@status)
    end

    # @return [Boolean] true if the job failed.
    def failed?
      @status == "FAILED"
    end
  end

  # Account quota information.
  class Quota
    attr_reader :plan, :included_hours, :credits_balance_hours, :reset_date

    def initialize(plan:, included_hours:, credits_balance_hours:, reset_date:)
      @plan = plan
      @included_hours = included_hours
      @credits_balance_hours = credits_balance_hours
      @reset_date = reset_date
    end
  end

  # Paginated list of jobs.
  class JobPage
    attr_reader :jobs, :next_cursor

    def initialize(jobs:, next_cursor:)
      @jobs = jobs
      @next_cursor = next_cursor
    end

    # @return [Boolean] true if there are more pages.
    def more?
      !@next_cursor.nil?
    end
  end

  # @api private
  module Parsers
    module_function

    def parse_job(data)
      Job.new(
        id: data["jobId"].to_s,
        status: data["status"].to_s,
        filename: data["originalFilename"].to_s,
        created_at: data["createdAt"].to_s,
        eta_seconds: data["estimatedCompletionTimeSeconds"],
        raw: data
      )
    end

    def parse_result(data)
      processed = data["processedData"] || {}
      scenes = (processed["scenes"] || []).map do |s|
        Scene.new(
          description: s["description"].to_s,
          end_time: s["endTs"].to_f,
          objects: Array(s["objects"])
        )
      end
      transcript = (processed["transcript"] || []).map do |t|
        TranscriptSegment.new(
          start_time: t["StartTime"].to_f,
          end_time: t["EndTime"].to_f,
          text: t["Text"].to_s
        )
      end

      ProcessingResult.new(
        job_id: data["jobId"].to_s,
        status: data["status"].to_s,
        filename: data["originalFilename"].to_s,
        duration: processed["length"].to_f,
        scenes: scenes,
        transcript: transcript,
        created_at: data["createdAt"].to_s,
        raw: data
      )
    end

    def parse_quota(data)
      Quota.new(
        plan: data["plan"].to_s,
        included_hours: data["includedHours"].to_f,
        credits_balance_hours: data["creditsBalanceHours"].to_f,
        reset_date: data["resetDate"]
      )
    end
  end
end
