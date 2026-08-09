# Plan: WPT / html5lib-tests Fixture Strategy and Benchmark Harness for M2

> Produced by a wave-1 performance research agent. Read-only; no files changed.

## Key facts shaping the plan

1. **No WebDriver/BiDi driver exists in M2** — full protocol surfaces are an M4 concern. `quality/WPT_PLAN.md` allows the alternative: "a dedicated standards harness that preserves WPT semantics." M2 must use that clause literally.
2. **No Rust test-harness deps exist yet** (no criterion/proptest/insta in `Cargo.lock`); no `[workspace.dependencies]` table; serde versions already drift slightly per-crate.
3. **Existing conventions**: manifest-driven JSON fixtures with `schema_version`/provenance fields (`tests/fixtures/manifest.json`), `node --test` for JS, plain `cargo test -p <crate>` for Rust — no custom Rust test framework.
4. **`just test` runs `cargo test --workspace` unconditionally** — `test-changed` is currently just an alias, NOT actually diff-scoped despite `FAST_INNER_LOOP.md`'s aspirational example. Any new WPT/html5lib fixture tests not feature-gated will silently join the default run and blow the 1-8 minute budget.
5. **A separate JS benchmark harness already exists** (`benchmarks/harness/runner.mjs`) with a defined result schema (`schema_version`, `workload_id`, `latency_ms`, `resources`, `memory`, `verified`, `attempts`) and `benchmarks/reports/` sink — targets end-to-end protocol-level workloads, not Rust-internal micro-benchmarks.
6. `security/supply-chain-manifest.json` (name/version/source/license/purpose/integrity) and `TEST_DATA.md` (owner/purpose/license/provenance/version-hash per fixture) are two overlapping obligations any vendoring must satisfy.
7. `agents/BLOCKERS.md`'s closure rule + `TEST_STRATEGY.md`'s `deferred_validation` ledger: any in-scope-per-`WPT_PLAN.md` test not run now must be an explicit, dated, reviewable record — never a silent omission.

## 1. Vendoring mechanism — two curated copy-in lanes, no git submodule

**Rejected: full `git submodule` on `web-platform-tests/wpt`** — multi-GB, hundreds of thousands of files, brittle Windows sparse-checkout, mixed per-directory licensing (would vendor far more than the P0/P1 list, and most `testharness.js` reftests can't even execute yet with no BiDi driver).

**Lane A — `tests/html5lib-tests/`** (sibling to `tests/wpt/`, distinct upstream project/license): `PROVENANCE.md` (commit SHA, date, MIT text) + `manifest.json` + `tokenizer/*.test` (JSON) + `tree-construction/*.dat`, copied verbatim. Used directly by M2-T03/M2-T04 — no JS runtime needed, matches where those crates sit in the dependency graph (before the V8 bridge exists). This is what production Rust HTML parsers (e.g. html5ever) already test against.

**Lane B — `tests/wpt/<area>/`** for genuine WPT content, mirroring upstream path layout: `PROVENANCE.md`, `manifest.json` (`source_repo`, `source_revision`, `included_paths[]`, `excluded_paths_note`), `resources/testharness.js` (vendored unmodified), plus per-area dirs (`url/`, `dom/nodes/`, `css/selectors/`, `html/webappapis/scripting/`, `html/browsing-the-web/`, `fetch/`, `custom-elements/`, `shadow-dom/`, etc.) and per-task `selection/<task-id>.selection.yaml` (§3).

`manifest.json` shape matches the existing `tests/fixtures/manifest.json` convention (`schema_version`, `source_repo`, `source_revision`, `fetched_at`, `license_notice`, `included_paths`, `refresh_reproduction` command). Register both manifests in `security/supply-chain-manifest.json`. A `scripts/wpt/refresh.mjs` (Node, same family as `scripts/build/*.mjs`) is the **only** sanctioned way to bump `source_revision` — a deliberate reviewed diff, never automatic.

## 2. Rust test-harness pattern — two shapes by dependency-graph position

### 2a. Data-driven harness (no JS runtime) — M2-T03, M2-T04, and non-script cases in M2-T05/T10

`crates/<crate>/tests/*.rs` gated by a `wpt-subset` Cargo feature (so it never joins the unscoped `cargo test --workspace`). A shared dev-only support crate `crates/wpt-support` provides: selection-file loading, WPT/html5lib JSON/`.dat` deserialization, output normalization (`doubleEscaped` handling, `initialStates` expansion), and an aggregated-failure reporter (not first-fail `assert_eq!`) writing to `artifacts/wpt/<task-id>/`.

For M2-T05 (DOM mutation/property) and M2-T10 (selectors/XPath): true WPT tests are `testharness.js`-driven, unavailable at their point in the dependency graph (T10 only depends on T05, no V8 yet). Plan: convert the *specific* needed WPT assertions into a **golden-file oracle** — input HTML fixture + selector/XPath expression + expected matched node id/attr set, computed once against a pinned reference engine, stored at `tests/wpt/derived/<area>/*.json` with a `derived_from` field naming the exact upstream WPT file + commit + reference-engine version. This reuses the oracle model `quality/DIFFERENTIAL_TESTING.md` already specifies rather than inventing a new one. Full script-driven WPT for DOM/selectors is explicitly recorded as **deferred to M2-T14 or M3** (once the V8 bridge + native worker exist end-to-end) — a genuine documented deferral, not a silent gap.

### 2b. In-process `testharness.js` shim — M2-T11, M2-T12, M2-T13 (V8 bridge available)

Load real WPT test files unmodified directly into the engine (no WebDriver/BiDi, matching `WPT_PLAN.md`'s allowance): spin up an isolate with test snapshot + DOM bindings, run vendored `testharness.js`, install a result-collector (`add_result_callback`/`add_completion_callback`), run the WPT source file, drain `Vec<{name, status: PASS|FAIL|TIMEOUT|NOTRUN, message}>`. Needs no protocol layer, no WebDriver session, no navigation stack — cheapest way to get real WPT semantics inside `cargo test` at this milestone. Full navigation-driven/multi-window/cross-origin WPT tests stay out of reach until T02/T09 and the protocol layer exist — must be recorded as deferred, not attempted with this shim.

## 3. Priority-subset selection and enforcement

Turn `WPT_PLAN.md`'s tiering rule ("every expected failure links to capability status/issue and has owner/review date; do not blanket-skip directories") into a checked-in artifact per task, following this repo's existing `.agent-state/evidence/*.claim.json` discipline.

**`tests/wpt/selection/<task-id>.selection.yaml`**: `task_id`, `area`, `wpt_revision` (must match `manifest.json#source_revision`), `priority_tier`, `included[]` (path + rationale), `excluded_but_in_priority_tier[]` (path + reason + target_milestone + requirement_ids), `expected_failures[]` (path + reason + issue + owner + review_date), `result_artifact` path.

**Enforcement** — `scripts/wpt/check-selection.mjs` (wired into `just wpt-selection-check`): fails if a selection references a path not in the vendored manifest (no phantom scope); fails if any vendored path tagged **P0** in `WPT_PLAN.md` isn't accounted for in *some* task's `included` or `excluded_but_in_priority_tier` with full reason/milestone/requirement fields (no P0 file silently absent from every list); generates `tests/wpt/SELECTION_INDEX.md` rolling up every task's deferred entries — this becomes the WPT-specific slice of `QUALITY_STRATEGY.md`'s deferred-test inventory, letting an independent reviewer (per CLAUDE.md's review-separation rule) see every deliberately-deferred priority test with owner/milestone in one generated file instead of trusting a PR description.

## 4. Benchmark harness (M2-T07 cold-vs-snapshot; forward-compatible with M3's full-snapshot-vs-delta)

**Tool: `criterion.rs`** — workspace pins stable Rust 1.86 (nightly-only `#[bench]` unusable), criterion is the standard stable choice and already does warm-up/outlier-rejection/statistical-significance/distribution reporting (matches `PERFORMANCE_BENCHMARKS.md`'s "report distributions, don't optimize from intuition") rather than hand-rolling that logic.

Bridge criterion's own `target/criterion/` output into the **existing** benchmark schema/sink (don't create a second reporting pipeline): `scripts/bench/export-criterion.mjs` reads `estimates.json`/`sample.json`, emits `benchmarks/reports/<date>/<workload_id>.json` in the same shape `runWorkload()` already produces (`schema_version`, `workload_id`, `build_id`=git SHA, `environment_id`, plus criterion's mean/median/p95/stddev instead of one `latency_ms`). Same crate/bench-target pattern reused verbatim for M3's full-snapshot-vs-delta extraction benchmark later — just a new `[[bench]]` entry, no new tooling.

**Baseline/regression policy**: first good export becomes checked-in `benchmarks/reports/baselines/v8-startup.json`. Regression *checking* belongs to Tier 2/3 (not the 1-8min fast gate) — split into a *quick* form for the task's own fast gate (records a number, sanity-checks snapshot-is-faster-than-cold — literally what M2-T07's acceptance criterion "warm startup baseline is recorded" asks, not a pass/fail regression gate on day one) and a *full* form for scheduled runs comparing against the stored baseline via criterion's own significance test (not a naive percentage cutoff) plus an owner-set tolerance. Record every run's build id/host topology/pinning/warm-up count/sample count/**raw sample array** (not just aggregates) so disputes can be re-analyzed, not re-run. Both cold and snapshot benchmark arms must assert the same postcondition (fully-initialized isolate) inside the `iter()` closure, not just measure wall time — never publish a bare "Nx faster" number without also verifying both arms actually succeeded equivalently.

## 5. One reusable fast-gate convention for all 14 M2 tasks

Two `just` recipe families (closing the gap `FAST_INNER_LOOP.md`'s aspirational `AREA=<area>` examples left open):
```just
wpt-subset CRATE:            cargo test -p {{CRATE}} --features wpt-subset
wpt-subset-all:               cargo test --workspace --features wpt-subset          # Tier-2 scheduled
bench-native CRATE BENCH:      cargo bench -p {{CRATE}} --bench {{BENCH}} -- --quick
                               node scripts/bench/export-criterion.mjs --crate {{CRATE}} --bench {{BENCH}}
bench-native-full CRATE BENCH: cargo bench -p {{CRATE}} --bench {{BENCH}}
                               node scripts/bench/export-criterion.mjs --crate {{CRATE}} --bench {{BENCH}} --check-regression
```
Every M2 task's `Fast gate` should read as one of exactly these two forms, e.g. `just wpt-subset machina-html-tokenizer` (T03), `just wpt-subset machina-html-tree-builder` (T04), `just wpt-subset machina-dom-core` (T05), `just wpt-subset machina-selectors` (T10), `just wpt-subset machina-v8-bridge` (T11/T12/T13 via the §2b shim), `just bench-native machina-v8-bridge startup` (T07). Because `wpt-subset` is a Cargo *feature*, none of this leaks into the unscoped `just test` — the default CI run stays in budget; `wpt-subset-all`/`bench-native-full` are the explicit Tier-2/3 scheduled commands.

## Summary of concrete deliverables for the implementer (not created by this read-only task)

`tests/html5lib-tests/{tokenizer,tree-construction}/` + manifest/provenance (Lane A) · `tests/wpt/<area>/` + `resources/testharness.js` + manifest/provenance (Lane B) · `tests/wpt/selection/<task-id>.selection.yaml` per task + `scripts/wpt/check-selection.mjs` + generated `SELECTION_INDEX.md` · `tests/wpt/derived/<area>/*.json` golden oracles for pre-V8 tasks · `crates/wpt-support` dev-only support crate · `wpt-subset` feature on every consuming crate · `criterion` `[[bench]]` targets + `scripts/bench/export-criterion.mjs` + `benchmarks/reports/baselines/` · two new `justfile` recipe families · `security/supply-chain-manifest.json` entries for both vendored sets.

## Files reviewed

`planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md` (all 14 tasks) · `quality/WPT_PLAN.md`, `TEST_STRATEGY.md`, `QUALITY_STRATEGY.md`, `FAST_INNER_LOOP.md`, `PERFORMANCE_BENCHMARKS.md`, `DIFFERENTIAL_TESTING.md`, `FUZZING.md`, `TEST_DATA.md` · `agents/BLOCKERS.md`, `TASK_PACKET_TEMPLATE.md` · `tests/fixtures/manifest.json`, `scripts/test/fixture-server.mjs`, `scripts/test/m1-compatibility-smoke.test.mjs` · `Cargo.toml` + `crates/native-core` + `crates/command-model/tests/consumer_typecheck.rs` · `justfile`, `scripts/build/run.mjs` · `benchmarks/harness/runner.mjs`, `benchmarks/corpus/manifest.json` · `protocols/CAPABILITY_MATRIX.md`, `security/SUPPLY_CHAIN.md`, `research/TESTING_RATIONALE.md`, `research/SOURCES.md`.
