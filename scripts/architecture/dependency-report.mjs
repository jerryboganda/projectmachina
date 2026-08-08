import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { findBoundaryViolations } from "./check-boundaries.mjs";

const root = fileURLToPath(new URL("../..", import.meta.url));
const policy = JSON.parse(
  await readFile(join(root, "architecture/boundary-policy.json"), "utf8")
);
const violations = await findBoundaryViolations(root, policy);
const report = {
  schema_version: "0.1.0",
  generated_by: "scripts/architecture/dependency-report.mjs",
  policy_version: policy.version,
  rules: policy.rules.map((rule) => ({
    id: rule.id,
    roots: rule.roots,
    forbidden_patterns: rule.forbidden_patterns
  })),
  violations
};

await mkdir(join(root, "architecture"), { recursive: true });
await writeFile(
  join(root, "architecture/dependency-report.json"),
  `${JSON.stringify(report, null, 2)}\n`,
  "utf8"
);
if (violations.length > 0) {
  console.error(violations.join("\n"));
  process.exit(1);
}
console.log("architecture dependency report: generated");
