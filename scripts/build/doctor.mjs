import { access, readFile } from "node:fs/promises";
import { constants } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnProjectCommand } from "./command.mjs";

const root = fileURLToPath(new URL("../..", import.meta.url));
const toolchainManifest = await readFile(
  join(root, "toolchains/versions.toml"),
  "utf8"
);
const requiredFiles = [
  "Cargo.toml",
  "rust-toolchain.toml",
  "CMakeLists.txt",
  "package.json",
  "pnpm-workspace.yaml",
  "pnpm-lock.yaml",
  "justfile",
  "toolchains/versions.toml",
  "apps/console/package.json",
  "cpp/v8-bridge/CMakeLists.txt",
  "schemas/command-model/v0.1/command-model.json",
  "scripts/contracts/generate.mjs",
  "scripts/agent/task-registry.mjs",
  "crates/command-model/src/generated.rs",
  "packages/contracts-ts/src/command-model.ts",
  "scripts/agent/claims.mjs",
  "scripts/security/redact.mjs",
  "crates/telemetry/Cargo.toml",
  "tests/fixtures/manifest.json"
];

const failures = [];

for (const relativePath of requiredFiles) {
  try {
    await access(join(root, relativePath), constants.F_OK);
  } catch {
    failures.push(`missing required file: ${relativePath}`);
  }
}

const packageJson = JSON.parse(await readFile(join(root, "package.json"), "utf8"));
if (packageJson.packageManager !== "pnpm@9.15.0") {
  failures.push("package.json must pin pnpm@9.15.0");
}

const versions = new Map();
for (const command of ["git", "cargo", "rustc", "cmake", "ninja", "clang", "buf", "node", "pnpm"]) {
  const result = spawnProjectCommand(command, ["--version"], {
    cwd: root,
    encoding: "utf8",
    stdio: "pipe"
  });
  if (result.error || result.status !== 0) {
    failures.push(`required command is unavailable: ${command}`);
  } else {
    versions.set(command, result.stdout.trim());
  }

  const gitRoot = spawnProjectCommand("git", ["rev-parse", "--show-toplevel"], {
    cwd: root,
    encoding: "utf8",
    stdio: "pipe"
  });
  if (gitRoot.error || gitRoot.status !== 0) {
    failures.push("working directory is not inside a Git repository");
  } else if (resolve(gitRoot.stdout.trim()) !== resolve(root)) {
    failures.push(`Git root does not match project root: ${gitRoot.stdout.trim()}`);
  }
}

const nodeVersion = versions.get("node")?.match(/^v(\d+)\.(\d+)\./);
if (!nodeVersion || Number(nodeVersion[1]) < 20 || (Number(nodeVersion[1]) === 20 && Number(nodeVersion[2]) < 18) || Number(nodeVersion[1]) >= 23) {
  failures.push(`Node.js must satisfy >=20.18.0 <23 (found ${versions.get("node") ?? "unknown"})`);
}
if (versions.get("pnpm") !== "9.15.0") {
  failures.push(`pnpm must be exactly 9.15.0 (found ${versions.get("pnpm") ?? "unknown"})`);
}
const cmakeVersion = versions.get("cmake")?.match(/cmake version (\d+)\.(\d+)\./);
if (!cmakeVersion || Number(cmakeVersion[1]) < 3 || (Number(cmakeVersion[1]) === 3 && Number(cmakeVersion[2]) < 24)) {
  failures.push(`CMake must satisfy >=3.24 (found ${versions.get("cmake") ?? "unknown"})`);
}
if (versions.get("buf") !== "1.47.2") {
  failures.push(`Buf must be exactly 1.47.2 (found ${versions.get("buf") ?? "unknown"})`);
}
const clangVersion = versions.get("clang")?.match(/clang version (\d+)\./);
if (!clangVersion || Number(clangVersion[1]) < 18) {
  failures.push(`Clang must satisfy >=18 (found ${versions.get("clang") ?? "unknown"})`);
}

function manifestVersion(section, key) {
  const sectionMatch = toolchainManifest.match(
    new RegExp(`\\[${section}\\]([\\s\\S]*?)(?=\\n\\[|$)`)
  );
  return sectionMatch?.[1]
    ?.match(new RegExp(`^${key}\\s*=\\s*"([^"]+)"`, "m"))?.[1];
}

if (process.env.MACHINA_STRICT_TOOLCHAIN === "1" || process.argv.includes("--strict")) {
  const expected = {
    rust: manifestVersion("rust", "channel"),
    node: manifestVersion("node", "version"),
    cmake: manifestVersion("cmake", "version"),
    clang: manifestVersion("clang", "version"),
    ninja: manifestVersion("ninja", "version"),
    buf: manifestVersion("buf", "version")
  };
  const actual = {
    rust: versions.get("rustc")?.match(/rustc ([0-9.]+)/)?.[1] ?? "",
    node: versions.get("node")?.replace(/^v/, "") ?? "",
    cmake: versions.get("cmake")?.match(/cmake version ([0-9.]+)/)?.[1] ?? "",
    clang: versions.get("clang")?.match(/clang version ([0-9.]+)/)?.[1] ?? "",
    ninja: versions.get("ninja") ?? "",
    buf: versions.get("buf") ?? ""
  };
  for (const [name, expectedVersion] of Object.entries(expected)) {
    if (expectedVersion && actual[name] !== expectedVersion) {
      failures.push(
        `${name} must be exactly ${expectedVersion} in strict mode (found ${actual[name] || "unknown"})`
      );
    }
  }
}

if (failures.length > 0) {
  console.error(failures.join("\n"));
  process.exitCode = 1;
} else {
  console.log("Project Machina doctor: ready");
}
