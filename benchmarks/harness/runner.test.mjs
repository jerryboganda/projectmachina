import assert from "node:assert/strict";
import test from "node:test";
import { runWorkload, summarizeResults } from "./runner.mjs";

const workload = {
  id: "fixture-navigation",
  category: "navigation"
};

test("counts only verified workloads as successful", async () => {
  const success = await runWorkload({
    workload,
    runner: async () => ({ title: "Machina" }),
    verify: (value) => value.title === "Machina",
    buildId: "test-build",
    environmentId: "test-env"
  });
  const failed = await runWorkload({
    workload: { ...workload, id: "broken" },
    runner: async () => ({ title: "Wrong" }),
    verify: (value) => value.title === "Machina",
    buildId: "test-build",
    environmentId: "test-env"
  });
  assert.equal(success.success, true);
  assert.equal(failed.success, false);
  assert.equal(failed.failure, "postcondition_failed");
  assert.deepEqual(summarizeResults([success, failed]), {
    total: 2,
    verified_successes: 1,
    failed_or_unverified: 1,
    verified_throughput_count: 1
  });
});

test("classifies runner exceptions without success-shaped fallback", async () => {
  const result = await runWorkload({
    workload,
    runner: async () => {
      throw new Error("fixture unavailable");
    },
    verify: () => true,
    buildId: "test-build",
    environmentId: "test-env"
  });
  assert.equal(result.success, false);
  assert.equal(result.verified, false);
  assert.equal(result.failure, "fixture unavailable");
});

test("performs bounded retries and records memory metrics", async () => {
  let attempts = 0;
  const result = await runWorkload({
    workload: { ...workload, id: "retrying" },
    runner: async () => {
      attempts += 1;
      return { ready: attempts > 1 };
    },
    verify: (value) => value.ready,
    buildId: "test-build",
    environmentId: "test-env",
    maxRetries: 1
  });
  assert.equal(result.success, true);
  assert.equal(result.attempts, 2);
  assert.equal(result.retries, 1);
  assert.equal(typeof result.memory.peak_rss_bytes, "number");
});
