import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { claimTask, heartbeatTask, releaseTask } from "./claims.mjs";

const REHEARSAL_VERSION = "0.1.0";

export async function runAutonomousLoopRehearsal() {
  const root = await mkdtemp(join(tmpdir(), "machina-loop-"));
  const now = new Date("2026-08-09T01:00:00.000Z");
  const evidenceDirectory = join(root, ".agent-state", "evidence");
  await mkdir(evidenceDirectory, { recursive: true });

  try {
    const first = await claimTask({
      root,
      task: "REHEARSAL-A",
      agent: "rehearsal-a",
      branch: "agent/REHEARSAL-A-fixtures",
      worktree: "../machina-worktrees/REHEARSAL-A",
      writeScope: ["tests/fixtures/**"],
      allowNonGit: true,
      now
    });
    const second = await claimTask({
      root,
      task: "REHEARSAL-B",
      agent: "rehearsal-b",
      branch: "agent/REHEARSAL-B-benchmarks",
      worktree: "../machina-worktrees/REHEARSAL-B",
      writeScope: ["benchmarks/**"],
      allowNonGit: true,
      now
    });

    let overlapRejected = false;
    try {
      await claimTask({
        root,
        task: "REHEARSAL-C",
        agent: "rehearsal-c",
        branch: "agent/REHEARSAL-C-overlap",
        worktree: "../machina-worktrees/REHEARSAL-C",
        writeScope: ["tests/fixtures/manifest.json"],
        allowNonGit: true,
        now
      });
    } catch (error) {
      if (!(error instanceof Error) || !error.message.includes("overlaps active claim")) {
        throw error;
      }
      overlapRejected = true;
    }
    assert.equal(overlapRejected, true);

    const firstHeartbeat = await heartbeatTask({
      root,
      task: first.task_id,
      agent: first.agent_id,
      branch: first.branch,
      worktree: first.worktree,
      allowNonGit: true,
      now: new Date("2026-08-09T01:10:00.000Z")
    });
    const secondHeartbeat = await heartbeatTask({
      root,
      task: second.task_id,
      agent: second.agent_id,
      branch: second.branch,
      worktree: second.worktree,
      allowNonGit: true,
      now: new Date("2026-08-09T01:10:00.000Z")
    });

    const handoff = {
      schema_version: REHEARSAL_VERSION,
      task_id: first.task_id,
      branch: first.branch,
      worktree: first.worktree,
      current_commit: "local-uncommitted",
      claim: firstHeartbeat,
      completed: ["claim", "heartbeat"],
      remaining: ["independent-review", "release"],
      recommended_next_action: "resume from this file and release after review"
    };
    const handoffPath = join(evidenceDirectory, "REHEARSAL-A.handoff.json");
    await writeFile(handoffPath, `${JSON.stringify(handoff, null, 2)}\n`, "utf8");

    const resumed = JSON.parse(await readFile(handoffPath, "utf8"));
    assert.equal(resumed.claim.task_id, "REHEARSAL-A");
    assert.equal(resumed.remaining.includes("release"), true);

    const releasedFirst = await releaseTask({
      root,
      task: first.task_id,
      agent: first.agent_id,
      branch: first.branch,
      worktree: first.worktree,
      allowNonGit: true,
      reason: "independent review simulated",
      now: new Date("2026-08-09T01:20:00.000Z")
    });
    const releasedSecond = await releaseTask({
      root,
      task: second.task_id,
      agent: second.agent_id,
      branch: second.branch,
      worktree: second.worktree,
      allowNonGit: true,
      reason: "independent review simulated",
      now: new Date("2026-08-09T01:20:00.000Z")
    });

    return {
      schema_version: REHEARSAL_VERSION,
      overlap_rejected: overlapRejected,
      resumed_from_handoff: resumed.task_id === first.task_id,
      released: [releasedFirst.status, releasedSecond.status],
      handoff_verified: true
    };
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

if (process.argv[1]?.endsWith("rehearsal.mjs")) {
  runAutonomousLoopRehearsal()
    .then((result) => console.log(JSON.stringify(result, null, 2)))
    .catch((error) => {
      console.error(error instanceof Error ? error.message : String(error));
      process.exitCode = 1;
    });
}
