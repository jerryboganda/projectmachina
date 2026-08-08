import { readFile, readdir } from "node:fs/promises";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../..", import.meta.url));
const policy = JSON.parse(
  await readFile(join(root, "architecture/boundary-policy.json"), "utf8")
);
const sourceExtensions = new Set([".rs", ".ts", ".tsx", ".js", ".mjs", ".svelte"]);

async function sourceFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    if (entry.name === "node_modules" || entry.name === ".svelte-kit" || entry.name === "dist" || entry.name === "build" || entry.name === "target") {
      continue;
    }
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await sourceFiles(path)));
    } else if (sourceExtensions.has(entry.name.slice(entry.name.lastIndexOf(".")))) {
      files.push(path);
    }
  }
  return files;
}

const violations = [];
for (const rule of policy.rules) {
  for (const rootPath of rule.roots) {
    const absoluteRoot = join(root, rootPath);
    let files;
    try {
      files = await sourceFiles(absoluteRoot);
    } catch (error) {
      if (error?.code === "ENOENT") {
        continue;
      }
      throw error;
    }
    for (const file of files) {
      const content = await readFile(file, "utf8");
      for (const pattern of rule.forbidden_patterns) {
        if (content.toLowerCase().includes(pattern.toLowerCase())) {
          violations.push(
            `${rule.id}: ${relative(root, file).replaceAll("\\", "/")} contains ${pattern}`
          );
        }
      }
    }
  }
}

if (violations.length > 0) {
  console.error(violations.join("\n"));
  process.exit(1);
}
console.log("architecture boundary check: passed");
