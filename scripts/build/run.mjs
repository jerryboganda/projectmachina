import { fileURLToPath } from "node:url";
import { spawnProjectCommand } from "./command.mjs";

const root = fileURLToPath(new URL("../..", import.meta.url));
const task = process.argv[2];

const tasks = {
  build: [
    ["cargo", ["build", "--workspace"]],
    ["cmake", ["--preset", "release"]],
    ["cmake", ["--build", "--preset", "release"]],
    ["pnpm", ["--filter", "@machina/console", "build"]]
  ],
  check: [
    [process.execPath, ["scripts/contracts/check.mjs"]],
    [process.execPath, ["scripts/contracts/roundtrip.mjs"]],
    [process.execPath, ["scripts/architecture/check-boundaries.mjs"]],
    [process.execPath, ["scripts/security/check.mjs"]],
    [process.execPath, ["scripts/security/check-supply-chain.mjs"]],
    [process.execPath, ["scripts/security/check-requirements.mjs"]],
    ["cargo", ["check", "--workspace"]],
    ["pnpm", ["--filter", "@machina/console", "check"]]
  ],
  "fmt-check": [
    ["cargo", ["fmt", "--all", "--", "--check"]],
    ["pnpm", ["--filter", "@machina/console", "fmt-check"]]
  ],
  test: [
    [process.execPath, ["--test", "scripts/agent/claims.test.mjs"]],
    [process.execPath, ["--test", "scripts/agent/claims.worktree.test.mjs"]],
    [process.execPath, ["--test", "scripts/agent/task-registry.test.mjs"]],
    [process.execPath, ["--test", "scripts/agent/rehearsal.test.mjs"]],
    [process.execPath, ["--test", "scripts/security/redact.test.mjs"]],
    [process.execPath, ["--test", "scripts/security/scan-secrets.test.mjs"]],
    [process.execPath, ["--test", "scripts/test/fixture-server.test.mjs"]],
    [process.execPath, ["--test", "benchmarks/harness/runner.test.mjs"]],
    [process.execPath, ["--test", "scripts/evidence/manifest.test.mjs"]],
    ["cargo", ["test", "--workspace"]],
    ["pnpm", ["--filter", "@machina/console", "test"]]
  ]
};

if (!task || !tasks[task]) {
  console.error(`unknown build task: ${task ?? "<missing>"}`);
  process.exit(2);
}

for (const [command, args] of tasks[task]) {
  const result = spawnProjectCommand(command, args, {
    cwd: root,
    encoding: "utf8",
    stdio: "inherit"
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}
