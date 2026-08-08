import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../..", import.meta.url));
const composeFile = "deploy/compose/compose.yaml";
const projectName = "machina-local";

function assertLocalEnvironment() {
  const environment = process.env.MACHINA_ENV ?? "local";
  if (environment !== "local") {
    throw new Error(`local command refused outside MACHINA_ENV=local (got ${environment})`);
  }
}

function run(args) {
  const result = spawnSync("docker", ["compose", "--project-name", projectName, "--file", composeFile, ...args], {
    cwd: root,
    encoding: "utf8",
    shell: false,
    stdio: "inherit"
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

function main([command, ...args]) {
  assertLocalEnvironment();
  if (command === "up") {
    run(["up", "-d", ...args]);
    return;
  }
  if (command === "down") {
    run(["down", ...args]);
    return;
  }
  if (command === "health") {
    run(["ps"]);
    return;
  }
  if (command === "reset") {
    if (!args.includes("--confirm")) {
      throw new Error("local reset is destructive; pass --confirm explicitly");
    }
    run(["down", "--volumes", "--remove-orphans"]);
    return;
  }
  throw new Error("usage: local.mjs up|down|health|reset --confirm");
}

try {
  main(process.argv.slice(2));
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
