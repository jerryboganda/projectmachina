import assert from "node:assert/strict";
import test from "node:test";
import { Redactor, REDACTION_MARKER } from "./redact.mjs";

test("redacts exact and base64 encoded canary secrets", () => {
  const redactor = new Redactor();
  const canary = "machina-canary-2026";
  redactor.registerSecret(canary);
  const encoded = Buffer.from(canary, "utf8").toString("base64");
  const output = redactor.redactText(`secret=${canary} encoded=${encoded}`);
  assert.equal(output.includes(canary), false);
  assert.equal(output.includes(encoded), false);
  assert.equal(output.match(/\[REDACTED\]/g)?.length, 2);
});

test("redacts sensitive headers and URL query values", () => {
  const redactor = new Redactor();
  redactor.registerSecret("page@token");
  const encoded = encodeURIComponent("page@token");
  const output = redactor.redactText(
    `Authorization: Bearer page@token\nX-Auth-Token: page@token\nhttps://example.test/path?token=${encoded}&safe=1`
  );
  assert.match(output, /Authorization: \[REDACTED\]/);
  assert.match(output, /X-Auth-Token: \[REDACTED\]/);
  assert.match(output, /token=\[REDACTED\]&safe=1/);
});

test("redacts unknown and page-content fields while preserving safe metadata", () => {
  const redactor = new Redactor();
  const output = redactor.redactValue({
    authorization: "Bearer secret-value",
    status: "visible",
    page_content: "untrusted page content",
    nested: {
      password: "password-value",
      unknown: "unclassified content"
    }
  });
  assert.deepEqual(output, {
    authorization: REDACTION_MARKER,
    status: "visible",
    page_content: REDACTION_MARKER,
    nested: {
      password: REDACTION_MARKER,
      unknown: REDACTION_MARKER
    }
  });
});

test("rejects empty registered secrets", () => {
  const redactor = new Redactor();
  assert.throws(() => redactor.registerSecret(""), /non-empty strings/);
});
