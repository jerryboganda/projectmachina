import { readFile, readdir } from "node:fs/promises";
import { basename, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../..", import.meta.url));
const sourceExtensions = new Set([".rs", ".ts", ".tsx", ".js", ".mjs", ".svelte"]);
const CARGO_MANIFEST = "Cargo.toml";

// Crate package names in this repository are consistently "machina-<name>"
// (see any crates/*/Cargo.toml [package].name). Normalizing "_" to "-" and
// lowercasing lets a single forbidden-pattern string in the policy match a
// real Cargo.toml dependency key (hyphenated, e.g. "machina-native-core")
// and the equivalent Rust import path (underscored, e.g.
// "machina_native_core::Engine") without requiring the policy file to spell
// out both forms for every pattern.
function normalize(text) {
  return text.toLowerCase().replaceAll("_", "-");
}

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
    } else if (
      entry.name === CARGO_MANIFEST ||
      sourceExtensions.has(entry.name.slice(entry.name.lastIndexOf(".")))
    ) {
      files.push(path);
    }
  }
  return files;
}

// Deliberately not a full TOML parser: a line-oriented scan of
// [dependencies]/[build-dependencies] (and their target-conditional
// variants, e.g. `[target.'cfg(unix)'.dependencies]`) tables is sufficient
// to recover real Cargo dependency-graph edges (the package name keys) from
// every Cargo.toml in this workspace, which currently only use flat
// `key = { path = "..." }` / `key = "version"` entries. `[dev-dependencies]`
// and `[workspace.dependencies]` (a version catalog, not a real edge for any
// one crate) are intentionally excluded.
export function extractCargoDependencyNames(content) {
  const names = [];
  let currentTable = null;
  for (const rawLine of content.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (line.length === 0 || line.startsWith("#")) {
      continue;
    }
    const tableMatch = line.match(/^\[([^\]]+)\]$/);
    if (tableMatch) {
      currentTable = tableMatch[1].trim();
      continue;
    }
    if (!currentTable || currentTable === "workspace.dependencies") {
      continue;
    }
    const isDependencyTable =
      currentTable === "dependencies" ||
      currentTable === "build-dependencies" ||
      currentTable.endsWith(".dependencies") ||
      currentTable.endsWith(".build-dependencies");
    if (!isDependencyTable) {
      continue;
    }
    const keyMatch = line.match(/^([A-Za-z0-9_.-]+)\s*=/);
    if (keyMatch) {
      names.push(keyMatch[1]);
    }
  }
  return names;
}

function matchingExceptionPatterns(rule, relPath) {
  const exceptions = rule.allowed_exceptions ?? [];
  const patterns = new Set();
  for (const exception of exceptions) {
    if (exception.path === relPath) {
      for (const pattern of exception.patterns) {
        patterns.add(normalize(pattern));
      }
    }
  }
  return patterns;
}

export async function findBoundaryViolations(scanRoot, policy) {
  const violations = [];
  for (const rule of policy.rules) {
    for (const rootPath of rule.roots) {
      const absoluteRoot = join(scanRoot, rootPath);
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
        const relPath = relative(scanRoot, file).replaceAll("\\", "/");
        const exceptionPatterns = matchingExceptionPatterns(rule, relPath);

        if (basename(file) === CARGO_MANIFEST) {
          const dependencyNames = extractCargoDependencyNames(content);
          for (const pattern of rule.forbidden_patterns) {
            const normalizedPattern = normalize(pattern);
            if (exceptionPatterns.has(normalizedPattern)) {
              continue;
            }
            const match = dependencyNames.find((name) =>
              normalize(name).includes(normalizedPattern)
            );
            if (match) {
              violations.push(
                `${rule.id}: ${relPath} declares a dependency on "${match}" (forbidden: ${pattern})`
              );
            }
          }
          continue;
        }

        const normalizedContent = normalize(content);
        for (const pattern of rule.forbidden_patterns) {
          const normalizedPattern = normalize(pattern);
          if (exceptionPatterns.has(normalizedPattern)) {
            continue;
          }
          if (normalizedContent.includes(normalizedPattern)) {
            violations.push(`${rule.id}: ${relPath} contains ${pattern}`);
          }
        }
      }
    }
  }
  return violations;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const policy = JSON.parse(
    await readFile(join(root, "architecture/boundary-policy.json"), "utf8")
  );
  const violations = await findBoundaryViolations(root, policy);
  if (violations.length > 0) {
    console.error(violations.join("\n"));
    process.exit(1);
  }
  console.log("architecture boundary check: passed");
}
