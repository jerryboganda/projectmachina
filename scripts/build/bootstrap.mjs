import { fileURLToPath } from "node:url";
import { spawnProjectCommand } from "./command.mjs";

const root = fileURLToPath(new URL("../..", import.meta.url));

function run(command, args) {
  const result = spawnProjectCommand(command, args, {
    cwd: root,
    stdio: "inherit"
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

run("node", ["scripts/build/doctor.mjs"]);
run("pnpm", ["install", "--frozen-lockfile", "--ignore-scripts"]);
run("node", ["scripts/contracts/generate.mjs"]);
run("node", ["scripts/agent/task-registry.mjs"]);
run("cargo", ["check", "--workspace"]);
run("cmake", ["--preset", "release"]);
run("cmake", ["--build", "--preset", "release"]);
run("pnpm", ["--filter", "@machina/console", "check"]);
