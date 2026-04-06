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

  class AudioTrack
    attr_reader :file_name, :url, :download_token, :sync_mode, :offset_ms,
                :label, :per_channel_transcription, :channels

    def initialize(file_name:, url: nil, download_token: nil, sync_mode: nil, offset_ms: nil,
                   label: nil, per_channel_transcription: nil, channels: nil)
      @file_name = file_name
      @url = url
      @download_token = download_token
      @sync_mode = sync_mode
      @offset_ms = offset_ms
      @label = label
      @per_channel_transcription = per_channel_transcription
      @channels = channels
    end

    def to_h
      h = { fileName: @file_name }
      h[:url] = @url if @url
      h[:downloadToken] = @download_token if @download_token
      h[:syncMode] = @sync_mode if @sync_mode
      h[:offsetMs] = @offset_ms if @offset_ms
      h[:label] = @label if @label
      h[:perChannelTranscription] = @per_channel_transcription unless @per_channel_transcription.nil?
      h[:channels] = @channels if @channels
      h
    end
  end

  class AudioTrackTranscript
    attr_reader :track_index, :track_name, :language, :status, :transcript,
                :speakers, :error_message

    def initialize(track_index:, track_name:, language:, status:, transcript: [],
                   speakers: [], error_message: nil)
      @track_index = track_index
      @track_name = track_name
      @language = language
      @status = status
      @transcript = transcript
      @speakers = speakers
      @error_message = error_message
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
  #   PENDING_UPLOAD, PENDING_FETCH, INGEST_PROCESSING, INGEST_TRANSCODING,
  #   INGEST_COMPLETED, VIDEO_PROCESSING, VIDEO_COMPLETED, VIDEO_COMPLETED_NO_SCENES,
  #   VISION_PROCESSING, VISION_COMPLETED, FAILED_FETCH, etc.
  class Job
    attr_reader :id, :status, :filename, :created_at, :eta_seconds,
                :audio_track_count, :audio_tracks_completed, :audio_track_names, :raw

    def initialize(id:, status:, filename:, created_at:, eta_seconds: nil,
                   audio_track_count: nil, audio_tracks_completed: nil, audio_track_names: [], raw: {})
      @id = id
      @status = status
      @filename = filename
      @created_at = created_at
      @eta_seconds = eta_seconds
      @audio_track_count = audio_track_count
      @audio_tracks_completed = audio_tracks_completed
      @audio_track_names = audio_track_names
      @raw = raw
    end

    def terminal?
      complete? || failed?
    end

    def complete?
      %w[VISION_COMPLETED VIDEO_COMPLETED_NO_SCENES].include?(@status)
    end

    def failed?
      @status.include?("FAILED")
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

  class BatchClip
    attr_reader :source_url, :file_name, :download_token, :provider

    def initialize(source_url:, file_name: nil, download_token: nil, provider: nil)
      @source_url = source_url
      @file_name = file_name
      @download_token = download_token
      @provider = provider
    end

    def to_h
      h = { sourceUrl: @source_url }
      h[:fileName] = @file_name if @file_name
      h[:downloadToken] = @download_token if @download_token
      h[:provider] = @provider if @provider
      h
    end
  end

  class BatchResult
    attr_reader :batch_id, :mode, :jobs

    def initialize(batch_id:, mode:, jobs:)
      @batch_id = batch_id
      @mode = mode
      @jobs = jobs
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
        audio_track_count: data["audioTrackCount"],
        audio_tracks_completed: data["audioTracksCompleted"],
        audio_track_names: Array(data["audioTrackNames"]),
        raw: data
      )
    end

    def parse_audio_track_transcript(data)
      transcript = (data["transcript"] || []).map do |t|
        TranscriptSegment.new(
          start_time: t["StartTime"].to_f,
          end_time: t["EndTime"].to_f,
          text: t["Text"].to_s
        )
      end

      AudioTrackTranscript.new(
        track_index: data["trackIndex"].to_i,
        track_name: data["trackName"].to_s,
        language: data["language"].to_s,
        status: data["status"].to_s,
        transcript: transcript,
        speakers: Array(data["speakers"]),
        error_message: data["errorMessage"]
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
        plan: data["currentPlan"].to_s,
        included_hours: data["includedHours"].to_f,
        credits_balance_hours: data["creditsBalanceHours"].to_f,
        reset_date: data["resetDate"]
      )
    end
  end
end
