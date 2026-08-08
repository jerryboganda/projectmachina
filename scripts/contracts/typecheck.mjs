import { existsSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = fileURLToPath(new URL("../..", import.meta.url));
const tscCandidates = [
  process.env.TSC,
  join(root, "node_modules/typescript/bin/tsc"),
  join(root, "apps/console/node_modules/typescript/bin/tsc"),
  join(root, "packages/contracts-ts/node_modules/typescript/bin/tsc")
].filter(Boolean);
const tsc = tscCandidates.find((candidate) => existsSync(candidate));

if (!tsc) {
  console.error(
    "TypeScript consumer check requires the pinned workspace TypeScript binary; run pnpm install first."
  );
  process.exit(2);
}

const result = spawnSync(
  process.execPath,
  [tsc, "--project", join(root, "tests/contract/tsconfig.json")],
  {
    cwd: root,
    encoding: "utf8",
    shell: false,
    stdio: "inherit"
  }
);

if (result.error) {
  throw result.error;
}
if (result.status !== 0) {
  process.exit(result.status ?? 1);
}

console.log("TypeScript consumer type-check: passed");
