import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { runWorkload, summarizeResults } from "./runner.mjs";

const root = fileURLToPath(new URL("../..", import.meta.url));
const manifest = JSON.parse(
  await readFile(join(root, "benchmarks/corpus/manifest.json"), "utf8")
);
assert.equal(typeof manifest.version, "string");

async function runOnce() {
  const results = [];
  for (const workload of manifest.workloads) {
    results.push(
      await runWorkload({
        workload,
        buildId: manifest.build_id,
        environmentId: "local-fixture",
        maxRetries: workload.max_retries ?? 0,
        runner: async () => ({
          workload_id: workload.id,
          fixture_version: workload.fixture_version
        }),
        verify: (value) =>
          value?.workload_id === workload.id &&
          value?.fixture_version === workload.fixture_version
      })
    );
  }
  return results;
}

const first = await runOnce();
const second = await runOnce();
assert.equal(summarizeResults(first).verified_successes, manifest.workloads.length);
assert.equal(summarizeResults(second).verified_successes, manifest.workloads.length);
const stableShape = (results) =>
  results.map(({ workload_id, category, verified, success, retries }) => ({
    workload_id,
    category,
    verified,
    success,
    retries
  }));
assert.deepEqual(stableShape(second), stableShape(first));
const latencyRange = (results) => {
  const values = results.map((result) => result.latency_ms);
  return Math.max(...values) - Math.min(...values);
};
assert.equal(Number.isFinite(latencyRange(first)), true);
assert.equal(Number.isFinite(latencyRange(second)), true);
console.log("benchmark reproducibility smoke: passed");
