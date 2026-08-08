import { readFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../..", import.meta.url));
const highConfidencePatterns = [
  { name: "private key", pattern: /-----BEGIN [A-Z ]*PRIVATE KEY-----/ },
  { name: "GitHub token", pattern: /\bgh[pousr]_[A-Za-z0-9_]{20,}\b/ },
  { name: "AWS access key", pattern: /\bAKIA[0-9A-Z]{16}\b/ },
  { name: "Slack token", pattern: /\bxox[baprs]-[0-9A-Za-z-]{20,}\b/ },
  { name: "OpenAI-style key", pattern: /\bsk-[A-Za-z0-9]{20,}\b/ }
];

export function findSecretIndicators(content) {
  return highConfidencePatterns
    .filter(({ pattern }) => pattern.test(content))
    .map(({ name }) => name);
}

function trackedAndUntrackedFiles() {
  const result = spawnSync(
    "git",
    ["ls-files", "--cached", "--others", "--exclude-standard", "-z"],
    {
      cwd: root,
      encoding: "buffer",
      shell: false,
      stdio: "pipe"
    }
  );
  if (result.error || result.status !== 0) {
    throw new Error(result.stderr?.toString("utf8") || "git file inventory failed");
  }
  return result.stdout
    .toString("utf8")
    .split("\0")
    .filter(Boolean);
}

const failures = [];
for (const relativePath of trackedAndUntrackedFiles()) {
  if (
    relativePath.includes("node_modules/") ||
    relativePath.includes(".svelte-kit/") ||
    relativePath.includes("target/") ||
    relativePath.includes("build/")
  ) {
    continue;
  }
  let content;
  try {
    content = await readFile(`${root}\\${relativePath}`, "utf8");
  } catch {
    continue;
  }
  for (const indicator of findSecretIndicators(content)) {
    failures.push(`${relativePath}: ${indicator}`);
  }
}

if (failures.length > 0) {
  console.error(`secret scan failed:\n${failures.join("\n")}`);
  process.exit(1);
}
console.log("secret scan: passed");
