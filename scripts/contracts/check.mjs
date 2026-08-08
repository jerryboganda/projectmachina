import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { validate } from "./validator.mjs";

const root = fileURLToPath(new URL("../..", import.meta.url));
const generatedRoot = await mkdtemp(join(tmpdir(), "machina-contracts-"));
const result = spawnSync(process.execPath, ["scripts/contracts/generate.mjs"], {
  cwd: root,
  encoding: "utf8",
  stdio: "pipe",
  shell: false,
  env: {
    ...process.env,
    MACHINA_CONTRACT_OUTPUT_DIR: generatedRoot
  }
});

if (result.error || result.status !== 0) {
  process.stderr.write(result.stderr || result.error?.message || "contract generation failed\n");
  await rm(generatedRoot, { recursive: true, force: true });
  process.exit(result.status ?? 1);
}

const schema = JSON.parse(await readFile(join(root, "schemas/command-model/v0.1/command-model.json"), "utf8"));
const requiredOutputs = schema["x-machina-codegen"].generated_outputs;
const schemaText = (await readFile(join(root, "schemas/command-model/v0.1/command-model.json"), "utf8")).replace(
  /\r\n/g,
  "\n"
);
const sourceHash = createHash("sha256").update(schemaText).digest("hex");
const missing = [];
const mismatched = [];
for (const relativePath of requiredOutputs) {
  try {
    const [expected, actual] = await Promise.all([
      readFile(join(root, relativePath), "utf8"),
      readFile(join(generatedRoot, relativePath), "utf8")
    ]);
    if (expected !== actual) {
      mismatched.push(relativePath);
    }
  } catch {
    missing.push(relativePath);
  }
}

if (missing.length > 0) {
  console.error(`missing generated outputs:\n${missing.join("\n")}`);
  await rm(generatedRoot, { recursive: true, force: true });
  process.exit(1);
}

if (mismatched.length > 0) {
  console.error(
    `generated outputs are stale; run pnpm contract:generate:\n${mismatched.join("\n")}`
  );
  await rm(generatedRoot, { recursive: true, force: true });
  process.exit(1);
}

const fixture = JSON.parse(
  await readFile(join(root, "tests/contract/command-model.fixture.json"), "utf8")
);
const fixtureResult = validate(fixture, schema);
assert.equal(
  fixtureResult.valid,
  true,
  `command model fixture failed validation:\n${fixtureResult.errors.join("\n")}`
);

const compatibility = JSON.parse(
  await readFile(join(root, "tests/contract/compatibility-fixtures.json"), "utf8")
);
assert.equal(compatibility.schema_version, schema["x-machina-codegen"].version);
assert.deepEqual(compatibility.policy, schema["x-machina-versioning"]);

for (const { fixture: name, expected_valid: expectedValid } of compatibility.cases) {
  const compatibilityFixture = JSON.parse(
    await readFile(join(root, "tests/contract", name), "utf8")
  );
  const compatibilityResult = validate(compatibilityFixture, schema);
  assert.equal(
    compatibilityResult.valid,
    expectedValid,
    `${name} unexpectedly passed the v0.1 compatibility/constraint policy`
  );
  if (!expectedValid) {
    if (name.includes("field")) {
      assert.ok(
        compatibilityResult.errors.some((error) => error.includes("unknown property")),
        `${name} did not exercise strict unknown-field validation`
      );
    } else if (name.includes("constraint")) {
      assert.ok(
        compatibilityResult.errors.some((error) => error.includes("must be one of")),
        `${name} did not exercise named enum validation`
      );
    }
  }
}

for (const output of requiredOutputs) {
  const generated = await readFile(join(root, output), "utf8");
  assert.match(generated, new RegExp(`source_schema_sha256: ${sourceHash}`));
}

await rm(generatedRoot, { recursive: true, force: true });
console.log("command model contract check: passed");
