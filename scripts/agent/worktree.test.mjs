import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import {
  createTaskWorktree,
  inspectWorktree,
  removeTaskWorktree
} from "./worktree.mjs";

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

test("creates, inspects, and removes a real task worktree and branch", async () => {
  const root = await mkdtemp(join(tmpdir(), "machina-worktree-helper-"));
  const worktree = join(dirname(root), `${basename(root)}-task-worktree`);
  try {
    git(root, ["init", "-b", "main"]);
    git(root, ["config", "user.name", "Worktree Test"]);
    git(root, ["config", "user.email", "worktree-test@example.invalid"]);
    await writeFile(join(root, "README.txt"), "worktree test\n", "utf8");
    git(root, ["add", "README.txt"]);
    git(root, ["commit", "-m", "worktree baseline"]);

    const created = await createTaskWorktree({
      root,
      task: "M0-T02",
      branch: "agent/worktree-helper",
      worktree
    });
    assert.equal(created.worktree.branch, "agent/worktree-helper");
    assert.equal(created.clean, true);
    assert.equal((await readFile(join(worktree, "README.txt"), "utf8")).trim(), "worktree test");

    const inspected = inspectWorktree({ root, worktree, branch: "agent/worktree-helper" });
    assert.equal(inspected.clean, true);

    const removed = await removeTaskWorktree({
      root,
      worktree,
      branch: "agent/worktree-helper",
      force: true,
      deleteBranch: true
    });
    assert.deepEqual(
      { removed: removed.removed, branch_deleted: removed.branch_deleted },
      { removed: true, branch_deleted: true }
    );
    assert.throws(
      () => inspectWorktree({ root, worktree }),
      /registered worktree not found/
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
