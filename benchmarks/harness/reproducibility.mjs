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

function expectedResult(workload) {
  const results = {
    "fixture-navigation": { title: "Machina fixture" },
    "fixture-extraction": { text: "Mutated" },
    "fixture-form": { accepted: true }
  };
  return results[workload.id];
}

async function runOnce() {
  const results = [];
  for (const workload of manifest.workloads) {
    const expected = expectedResult(workload);
    assert.ok(expected, `missing benchmark postcondition for ${workload.id}`);
    results.push(
      await runWorkload({
        workload,
        buildId: manifest.build_id,
        environmentId: "local-fixture",
        maxRetries: workload.max_retries ?? 0,
        runner: async () => ({
          workload_id: workload.id,
          fixture_version: workload.fixture_version,
          ...expected
        }),
        verify: (value) =>
          value?.workload_id === workload.id &&
          value?.fixture_version === workload.fixture_version &&
          Object.entries(expected).every(([key, expectedValue]) =>
            Object.is(value?.[key], expectedValue)
          )
      })
    );
  }
  return results;
}

export async function runReproducibility() {
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
  return { first, second };
}

if (process.argv[1]?.endsWith("reproducibility.mjs")) {
  await runReproducibility();
  console.log("benchmark reproducibility smoke: passed");
}
