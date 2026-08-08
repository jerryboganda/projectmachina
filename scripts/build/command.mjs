import { spawnSync } from "node:child_process";

function quoteCmdArgument(argument) {
  const value = String(argument);
  if (/^[A-Za-z0-9_@./:=+-]+$/.test(value)) {
    return value;
  }
  return `"${value.replaceAll('"', '""')}"`;
}

export function spawnProjectCommand(command, args, options = {}) {
  const { cwd, encoding, stdio = "inherit" } = options;
  if (process.platform === "win32" && command === "pnpm") {
    const commandLine = ["pnpm", ...args].map(quoteCmdArgument).join(" ");
    return spawnSync(process.env.ComSpec ?? "cmd.exe", ["/d", "/s", "/c", commandLine], {
      cwd,
      encoding,
      shell: false,
      stdio
    });
  }
  return spawnSync(command, args, {
    cwd,
    encoding,
    shell: false,
    stdio
  });
}
