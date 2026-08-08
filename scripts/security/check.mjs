import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { Redactor } from "./redact.mjs";

const root = fileURLToPath(new URL("../..", import.meta.url));
const policy = JSON.parse(
  await readFile(join(root, "security/redaction-policy.json"), "utf8")
);
const redactor = new Redactor();
const canary = "machina-security-check-canary";
redactor.registerSecret(canary);
const sample = {
  authorization: `Bearer ${canary}`,
  url: `https://example.test/?token=${canary}`,
  nested: {
    page_content: "untrusted page content is not emitted by default"
  }
};
const redacted = redactor.redactValue(sample);
const serialized = JSON.stringify(redacted);

if (serialized.includes(canary) || redactor.containsRegisteredSecret(serialized)) {
  console.error("security redaction check failed: canary leaked");
  process.exit(1);
}
if (policy.redaction_marker !== "[REDACTED]") {
  console.error("security redaction check failed: policy marker mismatch");
  process.exit(1);
}
console.log("security redaction check: passed");
