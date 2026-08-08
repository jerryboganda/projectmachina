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
  if (process.env.DOCKER_HOST) {
    throw new Error("local command refuses an ambient DOCKER_HOST; unset it before continuing");
  }
  if (process.env.DOCKER_CONTEXT && process.env.DOCKER_CONTEXT !== "default") {
    throw new Error("local command requires the default Docker context");
  }
  const context = spawnSync("docker", ["context", "show"], {
    cwd: root,
    encoding: "utf8",
    shell: false,
    stdio: "pipe"
  });
  if (context.error || context.status !== 0) {
    throw new Error("Docker is unavailable; local lifecycle commands cannot proceed");
  }
  if (context.stdout.trim() !== "default") {
    throw new Error(`local command requires Docker context default (got ${context.stdout.trim()})`);
  }
  const endpoint = spawnSync(
    "docker",
    ["context", "inspect", "default", "--format", "{{.Endpoints.docker.Host}}"],
    {
      cwd: root,
      encoding: "utf8",
      shell: false,
      stdio: "pipe"
    }
  );
  if (endpoint.error || endpoint.status !== 0) {
    throw new Error("unable to inspect the default Docker endpoint");
  }
  const host = endpoint.stdout.trim().toLowerCase();
  if (host.startsWith("tcp://") || host.startsWith("ssh://") || host.startsWith("http://") || host.startsWith("https://")) {
    throw new Error(`local command refuses remote Docker endpoint: ${host}`);
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

function capture(args) {
  const result = spawnSync("docker", ["compose", "--project-name", projectName, "--file", composeFile, ...args], {
    cwd: root,
    encoding: "utf8",
    shell: false,
    stdio: "pipe"
  });
  if (result.error || result.status !== 0) {
    throw result.error ?? new Error(result.stderr || "Docker Compose command failed");
  }
  return result.stdout;
}

function main([command, ...args]) {
  assertLocalEnvironment();
  if (command === "up") {
    run(["up", "-d", "--wait", ...args]);
    return;
  }
  if (command === "down") {
    run(["down", ...args]);
    return;
  }
  if (command === "health") {
    const output = capture(["ps", "--all", "--format", "json"]);
    const parsed = JSON.parse(output);
    const services = Array.isArray(parsed) ? parsed : [parsed];
    const expectedServices = new Set(["postgres", "redis", "object-store"]);
    const names = new Set(services.map((service) => service.Service));
    if (
      services.length !== expectedServices.size ||
      [...expectedServices].some((service) => !names.has(service)) ||
      services.some((service) => {
      const state = String(service.State ?? "").toLowerCase();
      const health = String(service.Health ?? "").toLowerCase();
      return state !== "running" || (health && health !== "healthy");
      })
    ) {
      console.error(output);
      throw new Error("one or more local services are not healthy");
    }
    console.log(output);
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
