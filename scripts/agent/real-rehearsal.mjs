import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  claimTask,
  heartbeatTask,
  releaseTask
} from "./claims.mjs";
import {
  createTaskWorktree,
  inspectWorktrees,
  removeTaskWorktree
} from "./worktree.mjs";
import { readHandoff, writeHandoff } from "./handoff.mjs";

function git(root, args) {
  const result = spawnSync("git", args, {
    cwd: root,
    encoding: "utf8",
    shell: false,
    stdio: "pipe"
  });
  if (result.error || result.status !== 0) {
    throw new Error(result.stderr || result.error?.message || `git failed: ${args.join(" ")}`);
  }
  return result.stdout.trim();
}

async function runIteration(index) {
  const root = await mkdtemp(join(tmpdir(), `machina-loop-${index}-`));
  const worktreeA = join(dirname(root), `machina-loop-${index}-a`);
  const worktreeB = join(dirname(root), `machina-loop-${index}-b`);
  const branchA = `agent/rehearsal-${index}-a`;
  const branchB = `agent/rehearsal-${index}-b`;
  let claimA;
  let claimB;
  try {
    git(root, ["init", "-b", "main"]);
    git(root, ["config", "user.name", "Machina Rehearsal"]);
    git(root, ["config", "user.email", "rehearsal@example.invalid"]);
    await writeFile(join(root, "README.md"), "rehearsal baseline\n", "utf8");
    git(root, ["add", "README.md"]);
    git(root, ["commit", "-m", "rehearsal baseline"]);
    const baseCommit = git(root, ["rev-parse", "HEAD"]);

    await createTaskWorktree({
      root,
      task: `REHEARSAL-${index}-A`,
      branch: branchA,
      worktree: worktreeA,
      baseCommit
    });
    await createTaskWorktree({
      root,
      task: `REHEARSAL-${index}-B`,
      branch: branchB,
      worktree: worktreeB,
      baseCommit
    });

    claimA = await claimTask({
      root,
      task: `REHEARSAL-${index}-A`,
      agent: `rehearsal-${index}-a`,
      branch: branchA,
      worktree: worktreeA,
      writeScope: ["tests/a/**"],
      baseCommit,
      currentCommit: baseCommit,
      now: new Date("2026-08-09T02:00:00.000Z")
    });
    claimB = await claimTask({
      root,
      task: `REHEARSAL-${index}-B`,
      agent: `rehearsal-${index}-b`,
      branch: branchB,
      worktree: worktreeB,
      writeScope: ["tests/b/**"],
      baseCommit,
      currentCommit: baseCommit,
      now: new Date("2026-08-09T02:00:00.000Z")
    });

    await mkdir(join(worktreeA, "tests/a"), { recursive: true });
    await writeFile(join(worktreeA, "tests/a/result.txt"), "A\n", "utf8");
    git(worktreeA, ["add", "tests/a/result.txt"]);
    git(worktreeA, ["commit", "-m", `REHEARSAL-${index}-A implementation`]);

    await mkdir(join(worktreeB, "tests/b"), { recursive: true });
    await writeFile(join(worktreeB, "tests/b/result.txt"), "B\n", "utf8");
    git(worktreeB, ["add", "tests/b/result.txt"]);
    git(worktreeB, ["commit", "-m", `REHEARSAL-${index}-B implementation`]);

    claimA = await heartbeatTask({
      root,
      task: claimA.task_id,
      agent: claimA.agent_id,
      branch: claimA.branch,
      worktree: claimA.worktree,
      ownerToken: claimA.owner_token,
      now: new Date("2026-08-09T02:10:00.000Z")
    });
    claimB = await heartbeatTask({
      root,
      task: claimB.task_id,
      agent: claimB.agent_id,
      branch: claimB.branch,
      worktree: claimB.worktree,
      ownerToken: claimB.owner_token,
      now: new Date("2026-08-09T02:10:00.000Z")
    });

    await writeHandoff({
      root,
      taskId: claimA.task_id,
      agent: claimA.agent_id,
      branch: claimA.branch,
      worktree: claimA.worktree,
      baseCommit,
      currentCommit: git(worktreeA, ["rev-parse", "HEAD"]),
      claim: claimA,
      completed: ["implementation", "heartbeat"],
      remainingSteps: ["review", "merge", "release"],
      commands: ["git diff --check"],
      recommendedNextAction: "resume from durable handoff"
    });
    const handoff = await readHandoff({ root, taskId: claimA.task_id });
    assert.equal(handoff.claim.owner_token, undefined);

    git(root, ["merge", "--no-ff", branchA, "-m", `merge ${branchA}`]);
    git(root, ["merge", "--no-ff", branchB, "-m", `merge ${branchB}`]);
    assert.equal(
      (await readFile(join(root, "tests/a/result.txt"), "utf8")).replace(/\r\n/g, "\n"),
      "A\n"
    );
    assert.equal(
      (await readFile(join(root, "tests/b/result.txt"), "utf8")).replace(/\r\n/g, "\n"),
      "B\n"
    );

    await releaseTask({
      root,
      task: claimA.task_id,
      agent: claimA.agent_id,
      branch: claimA.branch,
      worktree: claimA.worktree,
      ownerToken: claimA.owner_token,
      reason: "reviewed and merged",
      now: new Date("2026-08-09T02:20:00.000Z")
    });
    await releaseTask({
      root,
      task: claimB.task_id,
      agent: claimB.agent_id,
      branch: claimB.branch,
      worktree: claimB.worktree,
      ownerToken: claimB.owner_token,
      reason: "reviewed and merged",
      now: new Date("2026-08-09T02:20:00.000Z")
    });
    await removeTaskWorktree({ root, worktree: worktreeA, branch: branchA, deleteBranch: true });
    await removeTaskWorktree({ root, worktree: worktreeB, branch: branchB, deleteBranch: true });
    assert.equal(inspectWorktrees(root).length, 1);
    return { iteration: index, merged: true, handoff_resumed: true };
  } finally {
    await rm(worktreeA, { recursive: true, force: true });
    await rm(worktreeB, { recursive: true, force: true });
    await rm(root, { recursive: true, force: true });
  }
}

export async function runRealRehearsal() {
  return [await runIteration(1), await runIteration(2)];
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  runRealRehearsal()
    .then((result) => console.log(JSON.stringify({ status: "passed", result }, null, 2)))
    .catch((error) => {
      console.error(error instanceof Error ? error.message : String(error));
      process.exitCode = 1;
    });
}
