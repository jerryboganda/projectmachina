import assert from "node:assert/strict";
import test from "node:test";
import { runRealRehearsal } from "./real-rehearsal.mjs";

test("runs two real Git worktree loop rehearsals", async () => {
  const result = await runRealRehearsal();
  assert.deepEqual(result, [
    { iteration: 1, merged: true, handoff_resumed: true },
    { iteration: 2, merged: true, handoff_resumed: true }
  ]);
});
