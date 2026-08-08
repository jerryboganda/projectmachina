import assert from "node:assert/strict";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { claimTask, getClaimStoreRoot, releaseTask } from "./claims.mjs";

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
}

test("binds a claim to a registered Git worktree and branch", async () => {
  const root = await mkdtemp(join(tmpdir(), "machina-git-claims-"));
  const worktree = join(dirname(root), `${basename(root)}-worktree`);
  try {
    git(root, ["init", "-b", "main"]);
    git(root, ["config", "user.name", "Claim Test"]);
    git(root, ["config", "user.email", "claim-test@example.invalid"]);
    await writeFile(join(root, "README.txt"), "claim test\n", "utf8");
    git(root, ["add", "README.txt"]);
    git(root, ["commit", "-m", "test baseline"]);
    git(root, ["worktree", "add", worktree, "-b", "agent/claims-test"]);

    const claim = await claimTask({
      root,
      task: "M0-T02",
      agent: "worktree-test",
      branch: "agent/claims-test",
      worktree,
      writeScope: ["scripts/agent/**"],
      now: new Date("2026-08-09T01:00:00.000Z")
    });
    assert.equal(claim.status, "active");
    assert.equal(getClaimStoreRoot(root), join(root, ".git", "machina-claims"));
    const released = await releaseTask({
      root,
      task: "M0-T02",
      agent: "worktree-test",
      branch: "agent/claims-test",
      worktree,
      ownerToken: claim.owner_token,
      now: new Date("2026-08-09T01:01:00.000Z")
    });
    assert.equal(released.status, "released");

    await assert.rejects(
      claimTask({
        root,
        task: "M0-T04",
        agent: "wrong-worktree",
        branch: "agent/wrong-branch",
        worktree,
        writeScope: ["schemas/**"]
      }),
      /not registered on the claimed branch/
    );
  } finally {
    spawnSync("git", ["worktree", "remove", "--force", worktree], {
      cwd: root,
      stdio: "ignore",
      shell: false
    });
    await rm(root, { recursive: true, force: true });
  }
});
