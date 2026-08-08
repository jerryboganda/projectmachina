import assert from "node:assert/strict";
import test from "node:test";
import { findSecretIndicators, isScannablePath } from "./scan-secrets.mjs";

test("detects high-confidence credential formats", () => {
  const githubToken = `gh${"p_"}${"1".repeat(30)}`;
  const privateKeyHeader = `-----BEGIN ${"RSA PRIVATE KEY"}-----`;
  assert.deepEqual(findSecretIndicators(`token=${githubToken}`), [
    "GitHub token"
  ]);
  assert.deepEqual(findSecretIndicators(privateKeyHeader), [
    "private key"
  ]);
});

test("does not flag synthetic identifiers", () => {
  assert.deepEqual(findSecretIndicators("machina-security-check-canary"), []);
});

test("only excludes generated directory prefixes", () => {
  assert.equal(isScannablePath("scripts/build/doctor.mjs"), true);
  assert.equal(isScannablePath("build/output.bin"), false);
  assert.equal(isScannablePath("target/output.bin"), false);
  assert.equal(isScannablePath("build"), true);
});
