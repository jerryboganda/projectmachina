import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import {
  projectClaimEvidence,
  readHandoff,
  writeEvidenceProjection,
  writeHandoff
} from "./handoff.mjs";

test("writes resumable handoff and durable evidence projections", async () => {
  const root = await mkdtemp(join(tmpdir(), "machina-handoff-"));
  try {
    const handoff = await writeHandoff({
      root,
      taskId: "M0-T02",
      agent: "handoff-test",
      branch: "agent/M0-T02-handoff",
      worktree: "../machina-worktrees/M0-T02-handoff",
      baseCommit: "base-sha",
      currentCommit: "head-sha",
      objective: "resume the bounded task safely",
      acceptanceCriteria: ["state is machine-readable", "state has a human projection"],
      completed: ["implementation"],
      inProgress: ["review"],
      decisions: ["protected default branches remain forbidden"],
      commands: [{ command: "node --test", result: "passed" }],
      changedFiles: ["scripts/agent/**"],
      remainingSteps: ["independent review"],
      risks: ["terminal gate pending"],
      recommendedNextAction: "run the focused gate"
    });
    const resumed = await readHandoff({ root, taskId: "M0-T02" });
    assert.equal(resumed.identity.base_commit, "base-sha");
    assert.match(await readFile(handoff.markdown_path, "utf8"), /## Commands and results/);

    const evidence = await writeEvidenceProjection({
      root,
      taskId: "M0-T02",
      agent: "handoff-test",
      branch: "agent/M0-T02-handoff",
      worktree: "../machina-worktrees/M0-T02-handoff",
      status: "in-review",
      changedFiles: ["scripts/agent/handoff.mjs"],
      nextAction: "request independent review"
    });
    assert.equal(JSON.parse(await readFile(evidence.json_path, "utf8")).status, "in-review");

    const claimProjection = await projectClaimEvidence({
      root,
      claim: {
        task_id: "M0-T02",
        agent_id: "handoff-test",
        branch: "agent/M0-T02-handoff",
        worktree: "../machina-worktrees/M0-T02-handoff",
        write_scope: ["scripts/agent/**"],
        owner_token: "secret-owner-token",
        status: "released"
      },
      event: { type: "release", result: "recorded" }
    });
    assert.match(claimProjection.json_path, /\.claim\.json$/);
    assert.equal(
      (await readFile(claimProjection.json_path, "utf8")).includes("secret-owner-token"),
      false
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
