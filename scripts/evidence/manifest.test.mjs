import assert from "node:assert/strict";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import {
  createEvidenceManifest,
  serializeEvidenceManifest
} from "./manifest.mjs";

test("creates deterministic content-hashed evidence manifests", async () => {
  const root = await mkdtemp(join(tmpdir(), "machina-evidence-"));
  try {
    await writeFile(join(root, "a.txt"), "alpha", "utf8");
    await writeFile(join(root, "b.txt"), "beta", "utf8");
    const manifest = await createEvidenceManifest({
      root,
      taskId: "M0-T09",
      sourceCommit: "local-uncommitted",
      generatedAt: "2026-08-09T01:20:00.000Z",
      artifacts: [
        { artifact_id: "b", relative_path: "b.txt", classification: "restricted" },
        { artifact_id: "a", relative_path: "a.txt", classification: "restricted" }
      ]
    });
    assert.equal(manifest.artifacts[0].relative_path, "a.txt");
    assert.equal(manifest.artifacts[0].byte_length, 5);
    assert.match(manifest.artifacts[0].sha256, /^[a-f0-9]{64}$/);
    assert.equal(serializeEvidenceManifest(manifest).endsWith("\n"), true);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("rejects absolute, parent, duplicate, and directory artifacts", async () => {
  const root = await mkdtemp(join(tmpdir(), "machina-evidence-invalid-"));
  try {
    await writeFile(join(root, "a.txt"), "alpha", "utf8");
    await assert.rejects(
      createEvidenceManifest({
        root,
        taskId: "M0-T09",
        sourceCommit: "test",
        generatedAt: "now",
        artifacts: [
          { artifact_id: "a", relative_path: "../a.txt", classification: "restricted" }
        ]
      }),
      /escapes repository root/
    );
    await assert.rejects(
      createEvidenceManifest({
        root,
        taskId: "M0-T09",
        sourceCommit: "test",
        generatedAt: "now",
        artifacts: [
          { artifact_id: "a", relative_path: "a.txt", classification: "restricted" },
          { artifact_id: "a2", relative_path: "a.txt", classification: "restricted" }
        ]
      }),
      /paths must be unique/
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
