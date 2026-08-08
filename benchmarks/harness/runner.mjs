import { performance } from "node:perf_hooks";

export const BENCHMARK_SCHEMA_VERSION = "0.1.0";

function usageDelta(before, after) {
  return {
    user_cpu_us: after.userCPUTime - before.userCPUTime,
    system_cpu_us: after.systemCPUTime - before.systemCPUTime
  };
}

export async function runWorkload({
  workload,
  runner,
  verify,
  buildId,
  environmentId,
  maxRetries = 0
}) {
  if (!workload?.id || typeof runner !== "function" || typeof verify !== "function") {
    throw new Error("workload, runner, and verify are required");
  }
  if (!Number.isInteger(maxRetries) || maxRetries < 0) {
    throw new Error("maxRetries must be a non-negative integer");
  }

  const startedAt = new Date().toISOString();
  const startTime = performance.now();
  const startUsage = process.resourceUsage();
  const startMemory = process.memoryUsage();
  let value;
  let failure = null;
  let verified = false;
  let attempts = 0;
  while (!verified && attempts <= maxRetries) {
    attempts += 1;
    failure = null;
    try {
      value = await runner();
    } catch (error) {
      failure = error instanceof Error ? error.message : String(error);
    }
    if (failure === null) {
      try {
        verified = Boolean(await verify(value));
      } catch (error) {
        failure = error instanceof Error
          ? `verification_failed: ${error.message}`
          : `verification_failed: ${String(error)}`;
      }
    }
  }
  const endUsage = process.resourceUsage();
  const endMemory = process.memoryUsage();
  const result = {
    schema_version: BENCHMARK_SCHEMA_VERSION,
    workload_id: workload.id,
    category: workload.category,
    build_id: buildId,
    environment_id: environmentId,
    started_at: startedAt,
    latency_ms: Number((performance.now() - startTime).toFixed(3)),
    resources: usageDelta(startUsage, endUsage),
    memory: {
      rss_bytes_before: startMemory.rss,
      rss_bytes_after: endMemory.rss,
      heap_used_bytes_before: startMemory.heapUsed,
      heap_used_bytes_after: endMemory.heapUsed,
      peak_rss_bytes: Math.max(startMemory.rss, endMemory.rss)
    },
    attempts,
    retries: Math.max(0, attempts - 1),
    verified,
    success: verified,
    failure
  };

  if (!verified && result.failure === null) {
    result.failure = "postcondition_failed";
  }
  return result;
}

export function summarizeResults(results) {
  if (!Array.isArray(results)) {
    throw new TypeError("results must be an array");
  }
  const verified = results.filter((result) => result.verified === true);
  return {
    total: results.length,
    verified_successes: verified.length,
    failed_or_unverified: results.length - verified.length,
    verified_throughput_count: verified.length
  };
}
