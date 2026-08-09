# Evidence — tooling-fix-boundary-checker

## Identity

- Task: bounded tooling fix (not a milestone task) — fix
  `scripts/architecture/check-boundaries.mjs` gaps identified by a wave-1
  research agent in
  `.agent-state/design/M2-M1-contract-compatibility-checklist.md` section 4
  ("Crate-boundary directionality M2 must preserve").
- Branch: `agent/tooling-fix-boundary-checker`
- Worktree: `D:\Projects\Project Machina\.claude\worktrees\agent-ac7bb50b4e1ba3429`
  (the assigned worktree could only write inside itself; the branch was
  created/checked out in place rather than in a second worktree directory —
  see Decisions below)
- Base commit: `f38692d3a27d6a7c453f49be7907510600cfe034` (`main` at claim time)
- Commit: `583f120807e26a9801ebf8a831752b6d26574e77`
- Pull request: opened from `agent/tooling-fix-boundary-checker` to `main`
  (not merged)

## Changed files (owned scope only)

- `scripts/architecture/check-boundaries.mjs`
- `scripts/architecture/check-boundaries.test.mjs`
- `architecture/boundary-policy.json`

No other paths were touched. `agents/CURRENT_STATE.md`, `WORK_QUEUE.md`,
`WAIVERS.md`, `BLOCKERS.md` were not modified.

## What was fixed

Read `architecture/boundary-policy.json` and
`scripts/architecture/check-boundaries.mjs` in full before changing either.
All three reported gaps are fixed:

1. **Missing reverse-direction rule.** Added a new rule
   `native-engine-outward-only` with `roots` covering `crates/native-core`
   and every native-side crate directory named in the task
   (`crates/dom`, `crates/html`, `crates/event-loop`, `crates/network`,
   `crates/navigation`, `crates/runtime-v8`, `crates/storage`,
   `crates/semantic`, `crates/extraction`, `crates/security-policy`,
   `crates/state-bridge`), forbidding references to
   `machina-protocol-http`, `machina-protocol-cdp`, `machina-protocol-bidi`,
   `machina-protocol-mcp`, `machina-control-plane`, `machina-scheduler`,
   `machina-worker-pool`, `machina-auth`, `machina-policy`. Directories that
   are still stub `.gitkeep`-only crates (all of the above except
   `native-core`) simply yield no files to scan today, which is expected
   and confirmed by the passing run below.

2. **Cargo.toml never scanned.** `sourceFiles()` now also collects any file
   named exactly `Cargo.toml` regardless of extension. A new
   `extractCargoDependencyNames()` does a line-oriented (not a full TOML
   parser — matches the task's "simple TOML-key-line scan is fine")
   scan of `[dependencies]`, `[build-dependencies]`, and their
   target-conditional variants (e.g. `[target.'cfg(unix)'.dependencies]`),
   extracting real dependency package-name keys.  `[dev-dependencies]` and
   `[workspace.dependencies]` (a version catalog, not a per-crate edge) are
   intentionally excluded. `findBoundaryViolations` checks these extracted
   package names against `forbidden_patterns`, separately from the
   whole-file substring scan used for `.rs`/`.ts`/etc. source files.

3. **Hyphen/underscore mismatch.** Chose the "normalize matching" option
   explicitly offered by the task instead of duplicating every pattern in
   two forms: both file content/dependency names and `forbidden_patterns`
   are normalized (`lowercase` + `_` → `-`) before substring comparison, so
   one policy pattern (`"native-core"`) now matches a Cargo.toml dependency
   key (`machina-native-core`) and the equivalent Rust import path
   (`machina_native_core::...`) alike.

## Decisions and assumptions

- **Worktree mechanics.** The harness's Bash/Write/Edit tools reject any
  path outside the agent's originally assigned worktree directory
  (`.claude/worktrees/agent-ac7bb50b4e1ba3429`), even for `git worktree add`
  targets. I could not create and use a second worktree directory as the
  task's PROCESS section literally describes. Instead I checked out a fresh
  branch (`agent/tooling-fix-boundary-checker`, based on `main` at
  `f38692d3`) directly inside the already-assigned worktree, which was
  otherwise idle on a stale, unrelated branch
  (`worktree-agent-ac7bb50b4e1ba3429` at old commit `efd3007`, an M0-T01
  artifact) with a clean tree, so no other agent's work was at risk. All
  edits, the fast gate, and the commit happened in that single worktree.
- **Full package-name forbidden patterns for the new rule**, e.g.
  `"machina-scheduler"` rather than bare `"scheduler"`. Verified up front
  (via `Grep`) that bare words `policy`, `auth`, `scheduler`, `control-plane`
  already appear as ordinary English prose/identifiers inside
  `crates/native-core/src/lib.rs` today (doc comments like "worker/scheduler
  polling" and "never a protocol- or control-plane-layer", the imported
  `FallbackPolicy` type, and an `engine_policy` struct field). Bare-word
  patterns would have produced immediate false positives on day one. Using
  the full `machina-<name>` package identifier — this repository's
  universal crate-naming convention, confirmed by grepping every relevant
  `crates/*/Cargo.toml` `[package].name` — requires an actual crate
  reference (Cargo dependency key or `machina_<name>::` import path) and
  cannot match incidental prose. Regression-tested explicitly (see test
  "does not flag common English words...").
- **`allowed_exceptions` mechanism (new, minimal policy schema addition).**
  Fixing gap 3 (underscore matching) makes the checker newly able to see
  `crates/protocol-http`'s existing, documented, sanctioned dependency on
  `machina-native-core` — used only inside `#[cfg(test)] mod tests` in
  `crates/protocol-http/src/lib.rs` (verified by reading the file: the only
  `machina_native_core`/`machina_chromium_adapter` references are inside
  that test module; production code only depends on the generic
  `machina_command_bus::EngineAdapter` trait) and declared under
  `[dependencies]` (not `[dev-dependencies]`) in
  `crates/protocol-http/Cargo.toml`. The task explicitly requires this
  precedent not be flagged. Rather than silently special-casing test code
  everywhere (a much larger, unrequested change) or leaving the real bug
  half-fixed, I added a small, explicit, auditable
  `allowed_exceptions: [{path, patterns, reason}]` array scoped per rule,
  with two narrowly-targeted entries (documented with reasons referencing
  the design doc) for exactly `crates/protocol-http/Cargo.toml` and
  `crates/protocol-http/src/lib.rs`. This is a reversible, documented,
  narrow decision within task scope (AGENTS.md "Decision rules").
- Left the existing `protocol-adapter-inward-only` rule's
  `forbidden_patterns` values unchanged (`"native-core"`, `"runtime-v8"`,
  `"crates/dom"`, `"cpp/v8-bridge"`) — these are not common English words,
  so bare-word matching plus normalization is sufficient there; no false
  positives were found or introduced.
- Bumped `architecture/boundary-policy.json` `"version"` from `0.1.0` to
  `0.2.0` since the rule set changed (new rule, new field).

## Verification (scratch, not committed)

Wrote and ran a temporary Node script (outside the repo, in the scratchpad
temp directory, never staged/committed) that:

1. Created a synthetic `crates/native-core/src/lib.rs` containing
   `use machina_protocol_http::HttpCommandAdapter;` and a synthetic
   `crates/dom/Cargo.toml` declaring `machina-protocol-http` under
   `[dependencies]`, then ran `findBoundaryViolations` with the real,
   updated `architecture/boundary-policy.json`. Result: **2 violations**,
   both under the new `native-engine-outward-only` rule — one from the
   source-grep path, one from the Cargo.toml dependency-edge path. Confirms
   gaps 1–3 are actually fixed, not just structurally present.
2. Loaded the real, updated policy, stripped `allowed_exceptions` from every
   rule in memory (file on disk untouched), and ran `findBoundaryViolations`
   against the real repository root. Result: **exactly 2 violations**, both
   `protocol-adapter-inward-only` and both pointing at
   `crates/protocol-http/Cargo.toml` and `crates/protocol-http/src/lib.rs`
   for pattern `native-core` — proving the `allowed_exceptions` entries are
   suppressing a real match (not masking a no-op) and that nothing else in
   the repository is newly caught by the gap-3 fix.

Both scratch scripts were deleted after verification; no fake violation was
left committed anywhere.

## Commands run and results

```
$ node scripts/architecture/check-boundaries.mjs
architecture boundary check: passed

$ node --test scripts/architecture/check-boundaries.test.mjs
✔ reports forbidden protocol-to-engine imports (39.8278ms)
✔ allows an inward-only protocol adapter (26.1248ms)
✔ catches the underscored Rust import form of a hyphenated forbidden pattern (25.3735ms)
✔ catches a forbidden Cargo.toml [dependencies] edge even with no matching source text (18.4589ms)
✔ does not flag a Cargo.toml [dev-dependencies] entry (138.7569ms)
✔ reverse direction: reports a native-side crate importing a forbidden protocol crate (25.5075ms)
✔ reverse direction: reports a native-side crate depending on a forbidden protocol crate in Cargo.toml (17.7626ms)
✔ does not flag common English words that merely contain a forbidden crate name as a substring (22.025ms)
✔ allowed_exceptions suppresses a specific documented pattern in a specific file only (30.5169ms)
ℹ tests 9
ℹ pass 9
ℹ fail 0
```

## Acceptance mapping

- Gap 1 (no native→protocol rule): fixed by `native-engine-outward-only`;
  covered by two new tests (source-grep and Cargo.toml-edge, reverse
  direction).
- Gap 2 (Cargo.toml never scanned): fixed by `extractCargoDependencyNames` +
  Cargo.toml handling in `findBoundaryViolations`; covered by two new tests
  (positive match, `[dev-dependencies]` exclusion).
- Gap 3 (hyphen/underscore mismatch): fixed by uniform `normalize()`;
  covered by one new test using the exact underscored form from the design
  doc's example (`machina_native_core::NativeEngine`).
- No false positives against current repo state: `node
  scripts/architecture/check-boundaries.mjs` passes clean (see above); the
  known `protocol-http` → `native-core` test-only Cargo dependency is
  explicitly not flagged, verified both by the passing run and by the
  scratch "remove exceptions" check above, which shows removing the
  exception reproduces exactly that one known edge and nothing else.
- Bare-English-word false-positive risk (`policy`/`auth`/`scheduler`/
  `control-plane` already appearing in `crates/native-core` prose):
  addressed by using full package-name patterns; covered by a dedicated
  regression test.

## Deferred heavy validation

None applicable — this is a static text/TOML analysis script with no
compile step, network access, or runtime dependency; the fast gate above is
the complete validation surface for this change.

## Known risks and limitations

- The Cargo.toml scan is intentionally a line-oriented scan, not a full TOML
  parser: it does not currently handle the `[dependencies.<name>]`
  dotted-table-header form (only the flat `key = {...}` / `key = "version"`
  form used by every `Cargo.toml` in this workspace today). If a future
  crate's manifest switches to that style, its dependency edges would be
  invisible to this rule until the scan is extended — noted here for the
  next person who touches this file.
- `allowed_exceptions` is scoped by exact relative file path, not by AST
  region (e.g. `#[cfg(test)]`). If `crates/protocol-http/src/lib.rs`
  production (non-test) code starts referencing `machina_native_core`
  directly in the future, the exception as written would incorrectly
  suppress that new, real violation. This mirrors the existing tracked
  tech-debt item in the design doc (tighten the Cargo.toml dependency to
  `[dev-dependencies]`); doing that tightening is out of this task's scope
  (it owns test/policy/checker files only, not crate `Cargo.toml`s) but
  would let the exception be removed entirely.
- Did not run `node scripts/architecture/dependency-report.mjs` (which
  wraps `findBoundaryViolations` and writes
  `architecture/dependency-report.json`) since that script and its output
  artifact are outside this task's owned/write scope; it was not part of
  the required fast gate.
