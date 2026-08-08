import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { validate } from "./validator.mjs";

const root = fileURLToPath(new URL("../..", import.meta.url));
const schema = JSON.parse(
  await readFile(join(root, "schemas/command-model/v0.1/command-model.json"), "utf8")
);
const fixture = JSON.parse(
  await readFile(join(root, "tests/contract/command-model.fixture.json"), "utf8")
);

const fixtureResult = validate(fixture, schema);
assert.equal(
  fixtureResult.valid,
  true,
  `command model fixture failed validation:\n${fixtureResult.errors.join("\n")}`
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
assert.equal(validate(roundTripped, schema).valid, true);

const compatibility = JSON.parse(
  await readFile(join(root, "tests/contract/compatibility-fixtures.json"), "utf8")
);
assert.equal(compatibility.schema_version, schema["x-machina-codegen"].version);
assert.deepEqual(compatibility.policy, schema["x-machina-versioning"]);

for (const { fixture: name, expected_valid: expectedValid } of compatibility.cases) {
  const compatibilityFixture = JSON.parse(
    await readFile(join(root, "tests/contract", name), "utf8")
  );
  const result = validate(compatibilityFixture, schema);
  assert.equal(
    result.valid,
    expectedValid,
    `${name} validation result did not match its compatibility policy:\n${result.errors.join("\n")}`
  );
}

console.log("command model round-trip check: passed");
