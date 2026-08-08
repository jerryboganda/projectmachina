import assert from "node:assert/strict";
import { mkdir, mkdtemp, rm, utimes, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import {
  getClaimStoreRoot,
  recoverStaleLock,
  STALE_LOCK_MS
} from "./claims.mjs";

test("requires explicit stale-lock recovery and records a fencing audit", async () => {
  const root = await mkdtemp(join(tmpdir(), "machina-stale-lock-"));
  try {
    const lockPath = join(getClaimStoreRoot(root), "claims.lock");
    await mkdir(lockPath, { recursive: true });
    await writeFile(
      join(lockPath, "owner.json"),
      `${JSON.stringify({ lock_id: "old", fence: "old-fence", pid: 1 })}\n`,
      "utf8"
    );
    const old = new Date("2026-08-09T00:00:00.000Z");
    await utimes(lockPath, old, old);
    const recovered = await recoverStaleLock({
      root,
      actor: "orchestrator",
      reason: "process and worktree inspection completed",
      now: new Date(old.getTime() + STALE_LOCK_MS + 1)
    });
    assert.equal(recovered.recovered, true);
    assert.equal(recovered.audit.previous_owner.lock_id, "old");
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
