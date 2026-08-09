import assert from "node:assert/strict";
import test from "node:test";
import { runM1CompatibilitySmoke } from "./m1-compatibility-smoke.mjs";

test("runs the injected canonical command-core smoke matrix", async () => {
  const report = await runM1CompatibilitySmoke();
  assert.equal(report.status, "injected-command-core-smoke-passed");
  assert.equal(report.surface_mode, "labels only; not live HTTP/gRPC/SDK clients");
  assert.deepEqual(
    report.journeys.map((journey) => journey.surface),
    ["http", "grpc", "typescript", "python"]
  );
  assert.equal(report.fixture_form_sanity, "passed");
  assert.deepEqual(report.explicit_failures, [
    "COMMAND_CANCELLED",
    "UNSUPPORTED_CAPABILITY",
    "WORKER_LOST"
  ]);
  assert.equal(report.restart_reconciliation, "passed");
});
