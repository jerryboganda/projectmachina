import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../..", import.meta.url));
const schema = JSON.parse(
  await readFile(join(root, "schemas/command-model/v0.1/command-model.json"), "utf8")
);
const fixture = JSON.parse(
  await readFile(join(root, "tests/contract/command-model.fixture.json"), "utf8")
);

assert.equal(schema["x-machina-versioning"].unknown_fields, "reject");
assert.equal(schema["x-machina-versioning"].additive_fields, "new_version_or_extension_namespace");
assert.deepEqual(
  Object.keys(fixture).sort(),
  ["capability", "command", "event", "outcome"]
);
assert.equal(fixture.command.kind, "navigation.goto.v1");
assert.equal(fixture.command.payload.url.startsWith("https://"), true);
assert.equal(fixture.outcome.execution.fallback_used, false);
assert.equal(fixture.event.classification, "public");
assert.equal(fixture.capability.status, "native");

const roundTripped = JSON.parse(JSON.stringify(fixture));
assert.deepEqual(roundTripped, fixture);
console.log("command model round-trip check: passed");
