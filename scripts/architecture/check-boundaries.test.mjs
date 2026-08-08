import assert from "node:assert/strict";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { findBoundaryViolations } from "./check-boundaries.mjs";

const policy = {
  rules: [
    {
      id: "protocol-inward-only",
      roots: ["crates/protocol-cdp"],
      forbidden_patterns: ["native-core", "runtime-v8"]
    }
  ]
};

test("reports forbidden protocol-to-engine imports", async () => {
  const root = await mkdtemp(join(tmpdir(), "machina-boundaries-"));
  try {
    await mkdir(join(root, "crates/protocol-cdp/src"), { recursive: true });
    await writeFile(
      join(root, "crates/protocol-cdp/src/lib.rs"),
      "// forbidden native-core dependency marker\n",
      "utf8"
    );
    const violations = await findBoundaryViolations(root, policy);
    assert.equal(violations.length, 1);
    assert.match(violations[0], /protocol-inward-only/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("allows an inward-only protocol adapter", async () => {
  const root = await mkdtemp(join(tmpdir(), "machina-boundaries-ok-"));
  try {
    await mkdir(join(root, "crates/protocol-cdp/src"), { recursive: true });
    await writeFile(
      join(root, "crates/protocol-cdp/src/lib.rs"),
      "use machina_command_model::CommandEnvelope;\n",
      "utf8"
    );
    assert.deepEqual(await findBoundaryViolations(root, policy), []);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
