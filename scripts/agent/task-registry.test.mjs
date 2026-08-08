import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseMilestoneTasks } from "./task-registry.mjs";

test("parses task metadata and dependencies from a milestone packet", () => {
  const markdown = [
    "## M0-T01 — Bootstrap",
    "",
    "**Primary role:** platform",
    "**Dependencies:** none",
    "**Risk:** high",
    "**May run in parallel:** constrained",
    "",
    "## M0-T02 — Claims",
    "",
    "**Primary role:** platform",
    "**Dependencies:** M0-T01",
    "**Risk:** high",
    "**May run in parallel:** yes",
    "",
    "## M0-T03 — CI",
    "",
    "**Primary role:** platform",
    "**Dependencies:** M0-T01 through M0-T02",
    "**Risk:** high",
    "**May run in parallel:** yes"
  ].join("\n");
  const tasks = parseMilestoneTasks(markdown, "planning/test.md");
  assert.deepEqual(tasks[0].dependencies, []);
  assert.deepEqual(tasks[1].dependencies, ["M0-T01"]);
  assert.equal(tasks[1].primary_role, "platform");
  assert.deepEqual(tasks[2].dependencies, ["M0-T01", "M0-T02"]);
});

test("canonical milestone packets contain the complete task graph", async () => {
  const root = fileURLToPath(new URL("../..", import.meta.url));
  const files = [
    "MILESTONE_00_FOUNDATION_AND_GOVERNANCE.md",
    "MILESTONE_01_COMPATIBILITY_FIRST_PLATFORM.md",
    "MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md",
    "MILESTONE_03_NATIVE_WEB_APIS_AND_AUTOMATION.md",
    "MILESTONE_04_PROTOCOLS_AND_SDKS.md",
    "MILESTONE_05_DETERMINISTIC_AGENT_WORKFLOWS.md",
    "MILESTONE_06_SVELTE_CONSOLE_AND_DEVELOPER_EXPERIENCE.md",
    "MILESTONE_07_SECURITY_AND_CLOUD_OPERATIONS.md",
    "MILESTONE_08_COMPATIBILITY_PERFORMANCE_AND_RELIABILITY_HARDENING.md",
    "MILESTONE_09_FINAL_CERTIFICATION_AND_GENERAL_AVAILABILITY.md"
  ];
  let count = 0;
  for (const file of files) {
    const markdown = await readFile(join(root, "planning", file), "utf8");
    count += parseMilestoneTasks(markdown, `planning/${file}`).length;
  }
  assert.equal(count, 121);
});
