# frozen_string_literal: true

require "framequery"

# Create client (reads FRAMEQUERY_API_KEY env var if not passed)
client = FrameQuery::Client.new(api_key: "fq_your_api_key_here")

# 1. Process a local file (upload + wait for result)
result = client.process("interview.mp4")
puts "Duration: #{result.duration}s"
puts "Scenes: #{result.scenes.length}"
result.scenes.each do |scene|
  puts "  [#{scene.end_time}s] #{scene.description} — #{scene.objects.join(', ')}"
end
result.transcript.each do |seg|
  puts "  [#{seg.start_time}-#{seg.end_time}s] #{seg.text}"
end

# 2. Process from URL
result = client.process_url("https://cdn.example.com/video.mp4")
puts "URL video: #{result.duration}s, #{result.scenes.length} scenes"

# 3. Upload without waiting
job = client.upload("video.mp4")
puts "Job created: #{job.id} (#{job.status})"

# Check back later
job = client.get_job(job.id)
puts "Job status: #{job.status}"

# 4. Progress tracking (block-based)
result = client.process("video.mp4") do |job|
  eta = job.eta_seconds ? ", ETA: #{job.eta_seconds}s" : ""
  puts "  Status: #{job.status}#{eta}"
end

# 5. Check quota
quota = client.get_quota
puts "Plan: #{quota.plan}"
puts "Credits: #{quota.credits_balance_hours}h remaining"
puts "Included: #{quota.included_hours}h (resets #{quota.reset_date})"

# 6. List jobs
page = client.list_jobs(limit: 10, status: "COMPLETED")
page.jobs.each do |j|
  puts "  #{j.id}: #{j.status} — #{j.filename}"
end
puts "  ... more available" if page.more?
