import assert from "node:assert/strict";
import test from "node:test";
import { runAutonomousLoopRehearsal } from "./rehearsal.mjs";
import { runRealRehearsal } from "./real-rehearsal.mjs";

test("rehearses two isolated claims, handoff resume, and release", async () => {
  const result = await runAutonomousLoopRehearsal();
  assert.equal(result.overlap_rejected, true);
  assert.equal(result.resumed_from_handoff, true);
  assert.equal(result.handoff_verified, true);
  assert.deepEqual(result.released, ["released", "released"]);
});

test("rehearses two real Git worktree loops", async () => {
  assert.deepEqual(await runRealRehearsal(), [
    { iteration: 1, merged: true, handoff_resumed: true },
    { iteration: 2, merged: true, handoff_resumed: true }
  ]);
});
