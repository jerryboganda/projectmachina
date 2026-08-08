import assert from "node:assert/strict";
import test from "node:test";
import { findSecretIndicators } from "./scan-secrets.mjs";

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
