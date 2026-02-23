# frozen_string_literal: true

require "framequery"

client = FrameQuery::Client.new(api_key: "fq_your_api_key_here")

# Upload + wait for result
result = client.process("interview.mp4")
puts "#{result.duration}s, #{result.scenes.length} scenes"
result.scenes.each do |scene|
  puts "  [#{scene.end_time}s] #{scene.description} — #{scene.objects.join(', ')}"
end
result.transcript.each do |seg|
  puts "  [#{seg.start_time}-#{seg.end_time}s] #{seg.text}"
end

# From URL
result = client.process_url("https://cdn.example.com/video.mp4")

# Upload without waiting, then fetch later
job = client.upload("video.mp4")
job = client.get_job(job.id)
if job.complete?
  r = job.result
  puts "#{r.duration}s, #{r.scenes.length} scenes, #{r.transcript.length} transcript segments"
end

# Progress tracking
result = client.process("video.mp4") do |job|
  eta = job.eta_seconds ? ", ETA: #{job.eta_seconds}s" : ""
  puts "  #{job.status}#{eta}"
end

# Quota
quota = client.get_quota
puts "#{quota.plan}: #{quota.credits_balance_hours}h credits, #{quota.included_hours}h included (resets #{quota.reset_date})"

# List jobs (paginated)
page = client.list_jobs(limit: 10, status: "COMPLETED")
page.jobs.each { |j| puts "  #{j.id}: #{j.filename}" }
puts "  more..." if page.more?
