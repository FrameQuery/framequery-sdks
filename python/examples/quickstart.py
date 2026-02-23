"""Quickstart examples."""

from framequery import FrameQuery


def main() -> None:
    fq = FrameQuery(api_key="fq_your_api_key_here")

    # -- Process a local file --
    result = fq.process("interview.mp4")
    print(f"Duration: {result.duration}s")
    print(f"Scenes: {len(result.scenes)}")
    for scene in result.scenes:
        print(f"  [{scene.end_time}s] {scene.description} — {scene.objects}")
    print(f"Transcript segments: {len(result.transcript)}")
    for seg in result.transcript:
        print(f"  [{seg.start_time}-{seg.end_time}s] {seg.text}")

    # -- From URL --
    result = fq.process_url("https://cdn.example.com/video.mp4")
    print(f"Processed URL video: {result.duration}s, {len(result.scenes)} scenes")

    # -- Upload only --
    job = fq.upload("video.mp4")
    print(f"Job created: {job.id} (status: {job.status})")

    job = fq.get_job(job.id)
    print(f"Job status: {job.status}")

    # -- Progress callback --
    def on_progress(job):
        eta = f", ETA: {job.eta_seconds}s" if job.eta_seconds else ""
        print(f"  Status: {job.status}{eta}")

    result = fq.process("video.mp4", on_progress=on_progress)

    # -- Quota --
    quota = fq.get_quota()
    print(f"Plan: {quota.plan}")
    print(f"Credits: {quota.credits_balance_hours}h remaining")
    print(f"Included: {quota.included_hours}h (resets {quota.reset_date})")

    # -- List jobs --
    page = fq.list_jobs(limit=10, status="COMPLETED")
    for job in page.jobs:
        print(f"  {job.id}: {job.status} — {job.filename}")
    if page.has_more:
        next_page = fq.list_jobs(limit=10, cursor=page.next_cursor)
        print(f"  ... and {len(next_page.jobs)} more")


if __name__ == "__main__":
    main()
