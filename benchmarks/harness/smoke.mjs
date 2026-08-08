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

const results = [];
for (const workload of manifest.workloads) {
  const result = await runWorkload({
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
  });
  results.push(result);
}

const summary = summarizeResults(results);
assert.equal(summary.total, manifest.workloads.length);
assert.equal(summary.verified_successes, manifest.workloads.length);
console.log(JSON.stringify({ manifest: manifest.corpus_id, summary, results }, null, 2));
