import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = fileURLToPath(new URL("../..", import.meta.url));
const generatedRoot = await mkdtemp(join(tmpdir(), "machina-contracts-"));
const result = spawnSync(process.execPath, ["scripts/contracts/generate.mjs"], {
  cwd: root,
  encoding: "utf8",
  stdio: "pipe",
  shell: false,
  env: {
    ...process.env,
    MACHINA_CONTRACT_OUTPUT_DIR: generatedRoot
  }
});

if (result.error || result.status !== 0) {
  process.stderr.write(result.stderr || result.error?.message || "contract generation failed\n");
  await rm(generatedRoot, { recursive: true, force: true });
  process.exit(result.status ?? 1);
}

const schema = JSON.parse(await readFile(join(root, "schemas/command-model/v0.1/command-model.json"), "utf8"));
const requiredOutputs = schema["x-machina-codegen"].generated_outputs;
const missing = [];
const mismatched = [];
for (const relativePath of requiredOutputs) {
  try {
    const [expected, actual] = await Promise.all([
      readFile(join(root, relativePath), "utf8"),
      readFile(join(generatedRoot, relativePath), "utf8")
    ]);
    if (expected !== actual) {
      mismatched.push(relativePath);
    }
  } catch {
    missing.push(relativePath);
  }
}

if (missing.length > 0) {
  console.error(`missing generated outputs:\n${missing.join("\n")}`);
  await rm(generatedRoot, { recursive: true, force: true });
  process.exit(1);
}

if (mismatched.length > 0) {
  console.error(
    `generated outputs are stale; run pnpm contract:generate:\n${mismatched.join("\n")}`
  );
  await rm(generatedRoot, { recursive: true, force: true });
  process.exit(1);
}

await rm(generatedRoot, { recursive: true, force: true });
console.log("command model contract check: passed");
