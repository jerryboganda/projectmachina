import assert from "node:assert/strict";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { findBoundaryViolations } from "./check-boundaries.mjs";

const policy = {
  rules: [
    {
      id: "protocol-inward-only",
      roots: ["crates/protocol-cdp"],
      forbidden_patterns: ["native-core", "runtime-v8"]
    }
  ]
};

test("reports forbidden protocol-to-engine imports", async () => {
  const root = await mkdtemp(join(tmpdir(), "machina-boundaries-"));
  try {
    await mkdir(join(root, "crates/protocol-cdp/src"), { recursive: true });
    await writeFile(
      join(root, "crates/protocol-cdp/src/lib.rs"),
      "use machina-native-core::Engine;\n",
      "utf8"
    );
    const violations = await findBoundaryViolations(root, policy);
    assert.equal(violations.length, 1);
    assert.match(violations[0], /protocol-inward-only/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("allows an inward-only protocol adapter", async () => {
  const root = await mkdtemp(join(tmpdir(), "machina-boundaries-ok-"));
  try {
    await mkdir(join(root, "crates/protocol-cdp/src"), { recursive: true });
    await writeFile(
      join(root, "crates/protocol-cdp/src/lib.rs"),
      "use machina_command_model::CommandEnvelope;\n",
      "utf8"
    );
    assert.deepEqual(await findBoundaryViolations(root, policy), []);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("catches the underscored Rust import form of a hyphenated forbidden pattern", async () => {
  // Real Rust import paths use underscores (machina_native_core::...) while
  // policy patterns are written with hyphens (native-core); the checker
  // must normalize both sides so this is not missed.
  const root = await mkdtemp(join(tmpdir(), "machina-boundaries-underscore-"));
  try {
    await mkdir(join(root, "crates/protocol-cdp/src"), { recursive: true });
    await writeFile(
      join(root, "crates/protocol-cdp/src/lib.rs"),
      "use machina_native_core::NativeEngine;\n",
      "utf8"
    );
    const violations = await findBoundaryViolations(root, policy);
    assert.equal(violations.length, 1);
    assert.match(violations[0], /protocol-inward-only/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("catches a forbidden Cargo.toml [dependencies] edge even with no matching source text", async () => {
  const root = await mkdtemp(join(tmpdir(), "machina-boundaries-cargo-"));
  try {
    await mkdir(join(root, "crates/protocol-cdp"), { recursive: true });
    await writeFile(
      join(root, "crates/protocol-cdp/Cargo.toml"),
      [
        "[package]",
        'name = "machina-protocol-cdp"',
        "",
        "[dependencies]",
        'machina-native-core = { path = "../native-core" }',
        'serde_json = "1.0.151"'
      ].join("\n"),
      "utf8"
    );
    const violations = await findBoundaryViolations(root, policy);
    assert.equal(violations.length, 1);
    assert.match(violations[0], /protocol-inward-only/);
    assert.match(violations[0], /Cargo\.toml/);
    assert.match(violations[0], /machina-native-core/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("does not flag a Cargo.toml [dev-dependencies] entry", async () => {
  const root = await mkdtemp(join(tmpdir(), "machina-boundaries-cargo-dev-"));
  try {
    await mkdir(join(root, "crates/protocol-cdp"), { recursive: true });
    await writeFile(
      join(root, "crates/protocol-cdp/Cargo.toml"),
      [
        "[package]",
        'name = "machina-protocol-cdp"',
        "",
        "[dev-dependencies]",
        'machina-native-core = { path = "../native-core" }'
      ].join("\n"),
      "utf8"
    );
    assert.deepEqual(await findBoundaryViolations(root, policy), []);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("reverse direction: reports a native-side crate importing a forbidden protocol crate", async () => {
  const reversePolicy = {
    rules: [
      {
        id: "native-engine-outward-only",
        roots: ["crates/native-core"],
        forbidden_patterns: ["machina-protocol-http", "machina-scheduler"]
      }
    ]
  };
  const root = await mkdtemp(join(tmpdir(), "machina-boundaries-reverse-"));
  try {
    await mkdir(join(root, "crates/native-core/src"), { recursive: true });
    await writeFile(
      join(root, "crates/native-core/src/lib.rs"),
      "use machina_protocol_http::HttpCommandAdapter;\n",
      "utf8"
    );
    const violations = await findBoundaryViolations(root, reversePolicy);
    assert.equal(violations.length, 1);
    assert.match(violations[0], /native-engine-outward-only/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("reverse direction: reports a native-side crate depending on a forbidden protocol crate in Cargo.toml", async () => {
  const reversePolicy = {
    rules: [
      {
        id: "native-engine-outward-only",
        roots: ["crates/dom"],
        forbidden_patterns: ["machina-protocol-http"]
      }
    ]
  };
  const root = await mkdtemp(join(tmpdir(), "machina-boundaries-reverse-cargo-"));
  try {
    await mkdir(join(root, "crates/dom"), { recursive: true });
    await writeFile(
      join(root, "crates/dom/Cargo.toml"),
      [
        "[package]",
        'name = "machina-dom"',
        "",
        "[dependencies]",
        'machina-protocol-http = { path = "../protocol-http" }'
      ].join("\n"),
      "utf8"
    );
    const violations = await findBoundaryViolations(root, reversePolicy);
    assert.equal(violations.length, 1);
    assert.match(violations[0], /native-engine-outward-only/);
    assert.match(violations[0], /Cargo\.toml/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("does not flag common English words that merely contain a forbidden crate name as a substring", async () => {
  // Bare crate-name words like "scheduler"/"policy"/"auth" can appear in
  // ordinary prose/comments/identifiers (e.g. "worker/scheduler polling",
  // "FallbackPolicy", "engine_policy"). Forbidden patterns for the reverse
  // rule are written as full "machina-<name>" package identifiers so a
  // real crate reference is required, not an incidental English word.
  const reversePolicy = {
    rules: [
      {
        id: "native-engine-outward-only",
        roots: ["crates/native-core"],
        forbidden_patterns: ["machina-scheduler", "machina-policy", "machina-auth"]
      }
    ]
  };
  const root = await mkdtemp(join(tmpdir(), "machina-boundaries-no-false-positive-"));
  try {
    await mkdir(join(root, "crates/native-core/src"), { recursive: true });
    await writeFile(
      join(root, "crates/native-core/src/lib.rs"),
      [
        "/// Per-session health, suitable for worker/scheduler polling.",
        "use machina_command_bus::FallbackPolicy;",
        "struct Config { engine_policy: String }"
      ].join("\n"),
      "utf8"
    );
    assert.deepEqual(await findBoundaryViolations(root, reversePolicy), []);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("allowed_exceptions suppresses a specific documented pattern in a specific file only", async () => {
  const exceptionPolicy = {
    rules: [
      {
        id: "protocol-inward-only",
        roots: ["crates/protocol-http"],
        forbidden_patterns: ["native-core"],
        allowed_exceptions: [
          {
            path: "crates/protocol-http/src/lib.rs",
            patterns: ["native-core"],
            reason: "test-only usage, see design doc"
          }
        ]
      }
    ]
  };
  const root = await mkdtemp(join(tmpdir(), "machina-boundaries-exception-"));
  try {
    await mkdir(join(root, "crates/protocol-http/src"), { recursive: true });
    await writeFile(
      join(root, "crates/protocol-http/src/lib.rs"),
      "use machina_native_core::NativeEngine;\n",
      "utf8"
    );
    await mkdir(join(root, "crates/protocol-http/src/other"), { recursive: true });
    await writeFile(
      join(root, "crates/protocol-http/src/other/mod.rs"),
      "use machina_native_core::NativeEngine;\n",
      "utf8"
    );
    const violations = await findBoundaryViolations(root, exceptionPolicy);
    assert.equal(violations.length, 1);
    assert.match(violations[0], /other[\\/]mod\.rs/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
