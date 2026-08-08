import { createHash } from "node:crypto";
import { readFile, stat } from "node:fs/promises";
import { isAbsolute, join, relative, resolve } from "node:path";

export const EVIDENCE_SCHEMA_VERSION = "0.1.0";

function normalizeRelativePath(root, relativePath) {
  if (typeof relativePath !== "string" || relativePath.trim().length === 0) {
    throw new Error("evidence path must be a non-empty string");
  }
  const normalized = relativePath.replaceAll("\\", "/");
  if (isAbsolute(normalized) || /^[A-Za-z]:/.test(normalized)) {
    throw new Error(`evidence path must be repository-relative: ${relativePath}`);
  }
  if (normalized.split("/").some((segment) => segment === "..")) {
    throw new Error(`evidence path escapes repository root: ${relativePath}`);
  }
  const absolute = resolve(root, normalized);
  const checked = relative(resolve(root), absolute).replaceAll("\\", "/");
  if (checked.startsWith("../") || checked === "..") {
    throw new Error(`evidence path escapes repository root: ${relativePath}`);
  }
  return normalized;
}

export async function hashArtifact(root, relativePath) {
  const normalizedPath = normalizeRelativePath(root, relativePath);
  const absolutePath = join(root, normalizedPath);
  const metadata = await stat(absolutePath);
  if (!metadata.isFile()) {
    throw new Error(`evidence artifact is not a file: ${normalizedPath}`);
  }
  const contents = await readFile(absolutePath);
  return {
    relative_path: normalizedPath,
    sha256: createHash("sha256").update(contents).digest("hex"),
    byte_length: metadata.size
  };
}

export async function createEvidenceManifest({
  root,
  taskId,
  sourceCommit,
  generatedAt,
  artifacts
}) {
  if (!taskId || !sourceCommit || !generatedAt) {
    throw new Error("taskId, sourceCommit, and generatedAt are required");
  }
  if (!Array.isArray(artifacts) || artifacts.length === 0) {
    throw new Error("at least one evidence artifact is required");
  }
  const paths = artifacts.map((artifact) => artifact.relative_path);
  if (new Set(paths).size !== paths.length) {
    throw new Error("evidence artifact paths must be unique");
  }
  const hashedArtifacts = await Promise.all(
    artifacts.map(async (artifact) => ({
      artifact_id: artifact.artifact_id,
      classification: artifact.classification,
      ...(await hashArtifact(root, artifact.relative_path))
    }))
  );
  hashedArtifacts.sort((left, right) =>
    left.relative_path.localeCompare(right.relative_path)
  );
  return {
    schema_version: EVIDENCE_SCHEMA_VERSION,
    task_id: taskId,
    source_commit: sourceCommit,
    generated_at: generatedAt,
    artifacts: hashedArtifacts
  };
}

export function serializeEvidenceManifest(manifest) {
  return `${JSON.stringify(manifest, null, 2)}\n`;
}
