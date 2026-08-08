import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../..", import.meta.url));
const manifest = JSON.parse(
  await readFile(join(root, "security/supply-chain-manifest.json"), "utf8")
);
const appPackage = JSON.parse(
  await readFile(join(root, "apps/console/package.json"), "utf8")
);
const lockfile = await readFile(join(root, "pnpm-lock.yaml"), "utf8");

const requiredFields = ["name", "version", "source", "license", "purpose", "integrity"];
const failures = [];
const declaredPackages = new Set([
  ...Object.keys(appPackage.dependencies ?? {}),
  ...Object.keys(appPackage.devDependencies ?? {})
]);
const manifestPackages = new Set(
  manifest.direct_dependencies
    .filter((dependency) => dependency.source.includes("npmjs.com"))
    .map((dependency) => dependency.name)
);

for (const packageName of declaredPackages) {
  if (!manifestPackages.has(packageName)) {
    failures.push(`manifest is missing declared package: ${packageName}`);
  }
}

for (const dependency of manifest.direct_dependencies) {
  for (const field of requiredFields) {
    if (typeof dependency[field] !== "string" || dependency[field].trim().length === 0) {
      failures.push(`${dependency.name ?? "<unknown>"} is missing ${field}`);
    }
  }
  if (dependency.license.toLowerCase().includes("unknown")) {
    failures.push(`${dependency.name} has an unknown license`);
  }
  if (
    dependency.integrity.toLowerCase().includes("pending") ||
    dependency.integrity.toLowerCase().includes("unknown")
  ) {
    failures.push(`${dependency.name} has unresolved integrity metadata`);
  }
  if (
    dependency.source.includes("npmjs.com") &&
    !lockfile.includes(`${dependency.name}@${dependency.version}`)
  ) {
    failures.push(`${dependency.name}@${dependency.version} is absent from pnpm-lock.yaml`);
  }
}

if (failures.length > 0) {
  console.error(failures.join("\n"));
  process.exit(1);
}
console.log("supply-chain manifest check: passed");
