import FrameQuery from "framequery";

async function main() {
  const fq = new FrameQuery({ apiKey: "fq_your_api_key_here" });

  // 1. Process a local file (Node.js — upload + wait)
  const result = await fq.process("./interview.mp4");
  console.log(`Duration: ${result.duration}s`);
  console.log(`Scenes: ${result.scenes.length}`);
  for (const scene of result.scenes) {
    console.log(`  [${scene.endTime}s] ${scene.description} — ${scene.objects.join(", ")}`);
  }
  for (const seg of result.transcript) {
    console.log(`  [${seg.startTime}-${seg.endTime}s] ${seg.text}`);
  }

  // 2. Process from URL
  const urlResult = await fq.processUrl("https://cdn.example.com/video.mp4");
  console.log(`URL video: ${urlResult.duration}s, ${urlResult.scenes.length} scenes`);

  // 3. Upload without waiting
  const job = await fq.upload("./video.mp4");
  console.log(`Job created: ${job.id} (${job.status})`);

  const checked = await fq.getJob(job.id);
  console.log(`Job status: ${checked.status}`);

  // 4. Progress tracking
  const tracked = await fq.process("./video.mp4", {
    onProgress: (j) => {
      const eta = j.etaSeconds ? `, ETA: ${j.etaSeconds}s` : "";
      console.log(`  Status: ${j.status}${eta}`);
    },
  });

  // 5. Check quota
  const quota = await fq.getQuota();
  console.log(`Plan: ${quota.plan}`);
  console.log(`Credits: ${quota.creditsBalanceHours}h remaining`);

  // 6. List jobs
  const page = await fq.listJobs({ limit: 10, status: "COMPLETED" });
  for (const j of page.jobs) {
    console.log(`  ${j.id}: ${j.status} — ${j.filename}`);
  }
  if (page.hasMore) {
    console.log(`  ... more available (cursor: ${page.nextCursor})`);
  }
}

main().catch(console.error);
