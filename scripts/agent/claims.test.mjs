import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import test from "node:test";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  claimTask,
  heartbeatTask,
  normalizeScope,
  recoverExpiredTask,
  releaseTask,
  scopesOverlap
} from "./claims.mjs";

test("normalizes repository-relative scopes and rejects escapes", () => {
  const root = "D:/repo";
  assert.equal(normalizeScope(".\\crates\\dom\\**", root), "crates/dom/**");
  assert.equal(normalizeScope("security//redact.mjs", root), "security/redact.mjs");
  assert.throws(() => normalizeScope("../outside", root), /escapes repository root/);
  assert.throws(() => normalizeScope("D:\\outside", root), /repository-relative/);
});

test("preserves protected-main ownership rules", async () => {
  const root = await mkdtemp(join(tmpdir(), "machina-protected-branch-"));
  try {
    await assert.rejects(
      claimTask({
        root,
        task: "M0-T02",
        agent: "test-agent",
        branch: "main",
        worktree: "../machina-worktrees/M0-T02-test-agent",
        writeScope: ["scripts/agent/**"],
        allowNonGit: true
      }),
      /protected default branch/
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("detects overlapping literal and glob scopes conservatively", () => {
  assert.equal(scopesOverlap("crates/dom/**", "crates/dom/src/lib.rs"), true);
  assert.equal(scopesOverlap("crates/dom/**", "crates/html/**"), false);
  assert.equal(scopesOverlap("crates/dom/**", "crates/dom2/**"), false);
  assert.equal(scopesOverlap("**", "crates/html/**"), true);
  assert.equal(scopesOverlap("src/foo?.txt", "src/foo1.txt"), true);
  assert.equal(scopesOverlap("src/foo*", "src/foobar"), true);
  assert.equal(scopesOverlap("src/a[0-9].json", "src/a1.json"), true);
  assert.equal(
    scopesOverlap("src/Foo/**", "src/foo/bar.ts"),
    process.platform === "win32"
  );
});

test("rejects overlapping claims and preserves release evidence", async () => {
  const root = await mkdtemp(join(tmpdir(), "machina-claims-"));
  const now = new Date("2026-08-09T00:00:00.000Z");
  try {
    const first = await claimTask({
      root,
      task: "M0-T02",
      agent: "test-agent-a",
      branch: "agent/M0-T02-claims",
      worktree: "../machina-worktrees/M0-T02-test-agent-a",
      writeScope: ["scripts/agent/**"],
      allowNonGit: true,
      now
    });
    assert.equal(first.status, "active");
    await assert.rejects(
      claimTask({
        root,
        task: "M0-T03",
        agent: "test-agent-b",
        branch: "agent/M0-T03-ci",
        worktree: "../machina-worktrees/M0-T03-test-agent-b",
        writeScope: ["scripts/agent/claims.mjs"],
        allowNonGit: true,
        now
      }),
      /overlaps active claim M0-T02/
    );
    const heartbeat = await heartbeatTask({
      root,
      task: "M0-T02",
      agent: "test-agent-a",
      branch: first.branch,
      worktree: first.worktree,
      allowNonGit: true,
      now: new Date("2026-08-09T00:10:00.000Z")
    });
    assert.equal(heartbeat.heartbeat_at, "2026-08-09T00:10:00.000Z");
    await assert.rejects(
      heartbeatTask({
        root,
        task: "M0-T02",
        agent: "wrong-agent",
        branch: first.branch,
        worktree: first.worktree,
        allowNonGit: true,
        now: new Date("2026-08-09T00:10:00.000Z")
      }),
      /belongs to another agent/
    );
    const released = await releaseTask({
      root,
      task: "M0-T02",
      agent: "test-agent-a",
      branch: first.branch,
      worktree: first.worktree,
      allowNonGit: true,
      reason: "test complete",
      now: new Date("2026-08-09T00:11:00.000Z")
    });
    assert.equal(released.status, "released");
    const stored = JSON.parse(
      await readFile(join(root, ".agent-state/claims/M0-T02.json"), "utf8")
    );
    assert.equal(stored.release_reason, "test complete");
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("requires explicit recovery for expired claims", async () => {
  const root = await mkdtemp(join(tmpdir(), "machina-claims-expired-"));
  try {
    await claimTask({
      root,
      task: "M0-T02",
      agent: "test-agent-a",
      branch: "agent/M0-T02-claims",
      worktree: "../machina-worktrees/M0-T02-test-agent-a",
      writeScope: ["scripts/agent/**"],
      leaseMinutes: 1,
      graceMinutes: 1,
      allowNonGit: true,
      now: new Date("2026-08-09T00:00:00.000Z")
    });
    const recovered = await recoverExpiredTask({
      root,
      task: "M0-T02",
      actor: "orchestrator",
      reason: "lease expired with no process evidence",
      now: new Date("2026-08-09T00:03:00.000Z")
    });
    assert.equal(recovered.status, "recovered");
    await assert.rejects(
      heartbeatTask({
        root,
        task: "M0-T02",
        agent: "test-agent-a",
        branch: "agent/M0-T02-claims",
        worktree: "../machina-worktrees/M0-T02-test-agent-a",
        allowNonGit: true,
        now: new Date("2026-08-09T00:03:00.000Z")
      }),
      /not active/
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
