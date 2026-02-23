# frozen_string_literal: true

module FrameQuery
  class Scene
    attr_reader :description, :end_time, :objects

    def initialize(description:, end_time:, objects: [])
      @description = description
      @end_time = end_time
      @objects = objects
    end
  end

  class TranscriptSegment
    attr_reader :start_time, :end_time, :text

    def initialize(start_time:, end_time:, text:)
      @start_time = start_time
      @end_time = end_time
      @text = text
    end
  end

  # Returned by #process / #process_url after the job finishes.
  # `raw` holds the full API response if you need fields we don't wrap.
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

  # Represents a processing job. Status values:
  #   PENDING_ORCHESTRATION, FFMPEG_PROCESSING, VISION_API_PROCESSING,
  #   STT_PROCESSING, COMPLETED, COMPLETED_NO_SCENES, FAILED
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

    def terminal?
      %w[COMPLETED COMPLETED_NO_SCENES FAILED].include?(@status)
    end

    def complete?
      %w[COMPLETED COMPLETED_NO_SCENES].include?(@status)
    end

    def failed?
      @status == "FAILED"
    end

    # Parses processedData from the raw response. Returns nil unless complete.
    def result
      return nil unless complete?

      Parsers.parse_result(@raw)
    end
  end

  class Quota
    attr_reader :plan, :included_hours, :credits_balance_hours, :reset_date

    def initialize(plan:, included_hours:, credits_balance_hours:, reset_date:)
      @plan = plan
      @included_hours = included_hours
      @credits_balance_hours = credits_balance_hours
      @reset_date = reset_date
    end
  end

  class JobPage
    attr_reader :jobs, :next_cursor

    def initialize(jobs:, next_cursor:)
      @jobs = jobs
      @next_cursor = next_cursor
    end

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
