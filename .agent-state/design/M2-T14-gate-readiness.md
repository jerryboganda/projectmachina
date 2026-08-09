---
title: "M2-T14 Final Native-Corpus Gate — Readiness"
project: "Project Machina"
document_status: "read-only-scoping"
version: "1.0.0"
last_updated: "2026-08-09"
owners: "Program and Architecture"
purpose: "Pre-scope M2-T14 so it can start immediately once M2-T06/T07/T08/T09/T12 land, without a fresh research pass at claim time."
---

# M2-T14 gate readiness

Produced by a read-only prep pass. No crate, script, or `justfile` recipe was
modified. Sources read in full: `planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md`
(all 14 tasks, canonical — `planning/MILESTONE_02_NATIVE_ENGINE.md` is a
1.0.0 alias pointing at it), `.agent-state/design/M2-WPT-and-benchmark-harness-plan.md`,
every `.agent-state/evidence/M2-T*.md` currently in the repo (T01–T05, T10,
T11, T13), `scripts/test/m1-compatibility-smoke.mjs`, `agents/WORK_QUEUE.md`,
`agents/CURRENT_STATE.md`, `benchmarks/harness/runner.mjs`/`smoke.mjs`,
`justfile`, root `Cargo.toml`, and a directory scan of `tests/` and
`crates/`.

## 0. Task identity (verbatim from the milestone doc, lines 398–424)

- **Dependencies**: M2-T01 through M2-T13, M1-T09.
- **Deliverables**: package native worker with scheduler registration and
  canonical adapter; enable router `prefer-native` for declared foundation
  capabilities; run selected deterministic and approved corpus tasks,
  recording fallback/unsupported.
- **Acceptance criteria**: (1) at least the milestone target fixtures
  navigate, execute JS, query/extract and close natively; (2) unsupported
  capabilities return a structured miss and can route to Chromium when
  policy allows; (3) native/Chromium traces share the canonical outcome
  format.
- **Fast gate**: run native engine contract and M2 corpus suite; run worker
  crash/cancel/resource-limit integration tests.

## 1. Dependency status as of this pass

| Task | Status | Evidence |
|---|---|---|
| M2-T01 session/context/page | merged | `.agent-state/evidence/M2-T01.md` |
| M2-T02 network loader | merged | `.agent-state/evidence/M2-T02.md` |
| M2-T03 HTML tokenizer | merged | `.agent-state/evidence/M2-T03.md` |
| M2-T04 HTML tree builder | merged | `.agent-state/evidence/M2-T04.md` |
| M2-T05 DOM core | merged | `.agent-state/evidence/M2-T05.md` |
| M2-T06 V8 bridge | **not merged** — `agents/WORK_QUEUE.md` records it "in progress"; no `.agent-state/evidence/M2-T06.md` exists; `crates/runtime-v8` exists as a directory but was not read as part of this pass (out of scope: read-only prep, not a code check on an in-flight task) | — |
| M2-T07 V8 snapshot/bindings | **not started** (needs T06) | — |
| M2-T08 event loop | **not started** (needs T06/T07/T02) | — |
| M2-T09 navigation lifecycle | **not started** (needs T02/T04/T07/T08) | — |
| M2-T10 selectors/XPath | merged | `.agent-state/evidence/M2-T10.md` |
| M2-T11 event dispatch/focus | merged | `.agent-state/evidence/M2-T11.md` |
| M2-T12 fetch/XHR/cookies/storage | **not started** (needs T02/T07/T08/T09) | — |
| M2-T13 semantic index/markdown | merged | `.agent-state/evidence/M2-T13.md` |
| M1-T09 capability snapshots/routing | merged | `.agent-state/evidence/M1-T09.md` |

Eight of thirteen prerequisite M2 tasks are merged (T01/T02/T03/T04/T05/T10/
T11/T13). T06/T07/T08/T09/T12 remain, in that dependency order — this
matches `agents/WORK_QUEUE.md`'s own current accounting. **M2-T14 genuinely
cannot start (claim/branch/implement) until all five land**, because its
first deliverable ("package native worker... enable router `prefer-native`")
requires a working V8 bridge, event loop, and navigation lifecycle to exist
at all — there is no way to shortcut this with a partial gate.

## 2. Acceptance-criterion → satisfied-by-what checklist

| # | Acceptance criterion (verbatim) | Already satisfied today by | Depends on (not yet merged) |
|---|---|---|---|
| 1 | "At least the milestone target fixtures navigate, execute JS, query/extract and close natively." | Partial building blocks only: T01 (session/context/page lifecycle + resource accounting), T02 (streaming HTTP loader with redirect/compression/cancellation), T03+T04 (tokenizer → tree builder → DOM), T10 (selector/XPath query), T11 (event dispatch incl. `interaction.click.v1` wired end-to-end per `agents/WORK_QUEUE.md`'s "interaction.click.v1 is now wired end-to-end" note), T13 (markdown/semantic extraction). **None of these compose into an actual navigate→execute-JS→query→close flow yet** — there is no V8 execution (T06/T07), no event loop to drive promises/timers/network completions (T08), and no navigation state machine that streams a response into the parser and reaches load (T09). "Execute JS" specifically cannot be demonstrated at all until T06/T07 exist. | T06, T07, T08, T09 (all four; T12 not required for *this* sub-criterion but is required for the milestone's fetch/storage-bearing fixtures if any target fixture uses them) |
| 2 | "Unsupported capabilities return structured miss and can route to Chromium when policy allows." | The routing/fallback mechanism itself is proven at the M1 layer: M1-T09 (capability snapshots + policy-aware eligibility + structured routing decisions with both-engine evidence) is merged, and `crates/native-core`'s `DispatchError::unsupported` catch-all (visible in T01's evidence and exercised by `scripts/test/m1-compatibility-smoke.mjs`) already returns a typed `UNSUPPORTED_CAPABILITY` for any `CommandKind` the native engine doesn't yet implement. This is a **real, already-exercised mechanism**, not aspirational — the M1 compatibility smoke test's `failures.execute(...)` cases for unrecognized `CommandKind`s prove it end-to-end today. | T14's own work is to enable `prefer-native` for the *new* T06–T13 capabilities and verify each declared-but-not-yet-implemented one still falls through cleanly — the mechanism exists, the specific capability declarations for the new crates do not yet. |
| 3 | "Native/Chromium traces share the canonical outcome format." | The canonical outcome/trace format itself (M1-T10: bounded traces, scoped artifacts) and the command/event/error model (M1-T07/T08) are merged and already used identically by both `NativeEngine`/`ChromiumEngine` in `crates/native-core` (T01's evidence: "ChromiumEngine composes context/page identities... through the identical `LifecycleEngine`/`EngineSession` path as `NativeEngine`"). No new format work is needed; T14's job is to confirm the T06–T13 code paths populate the same `trace_ref`/`execution`/outcome shape `m1-compatibility-smoke.mjs` already asserts on, once real navigate/execute/query/close commands exist. | T06–T09, T12 (need real command handlers to trace) |

**Bottom line**: criterion 2 and 3's *mechanisms* are already built and
independently evidenced; only criterion 1 (and the "populate real traces for
the new capabilities" half of criteria 2/3) is blocked on T06/T07/T08/T09/T12.
There is no criterion M2-T14 can partially satisfy today with merged code —
its own deliverables ("package native worker," "enable router
`prefer-native`") are meaningless before a native worker can actually
navigate+execute+query+close at all.

## 3. Fast-gate readiness

- **"Run native engine contract and M2 corpus suite."** No `M2 corpus`
  fixture set exists yet as a named, runnable target — each merged M2 crate
  (T01–T05/T10/T11/T13) currently has its own crate-local hand-authored test
  suite (documented per-task above, e.g. T04's 37 tests, T10's 80, T13's 42)
  but there is no cross-crate "navigate a real fixture page end-to-end
  natively" corpus runner anywhere in the repo — because the pieces that
  would make that runnable (V8 execution, event loop, navigation) do not
  exist yet. This is expected, not a gap in T14's own scope: assembling that
  corpus **is** T14's deliverable, not a precondition for it.
- **"Run worker crash/cancel/resource-limit integration tests."**
  `crates/session`'s cascading cancellation and per-page resource hard
  limits (T01) are unit-tested today; there is no "worker" process/lane yet
  to crash-test (that's `crates/worker-pool`/`crates/scheduler`, listed in
  `crates/` but not confirmed built against a real native engine in this
  pass — out of scope to verify here since it's not an M2-T14 dependency
  named in the milestone doc). T14's own crash/cancel/resource-limit tests
  will need a real packaged native worker to exist first, which is exactly
  what T14 itself produces.

## 4. WPT harness — actually runnable today, or still a plan?

**Still entirely a plan. Verified directly, not inferred from the design
doc's own wording:**

- `tests/wpt/` contains exactly one file: `.gitkeep`. No vendored WPT
  content, no `manifest.json`, no `PROVENANCE.md`, no `selection/` directory.
- `find tests -iname "*html5lib*"` returns nothing — Lane A
  (`tests/html5lib-tests/`) from the plan does not exist either.
- `crates/wpt-support` (the plan's shared dev-only harness crate) does not
  exist — `ls crates/` lists 27 real crate directories and none is named
  `wpt-support`.
- No crate declares a `wpt-subset` Cargo feature — `grep -rn "wpt-subset"
  Cargo.toml crates/*/Cargo.toml` returns nothing.
- `justfile` has no `wpt-subset`, `wpt-subset-all`, `bench-native`, or
  `bench-native-full` recipe — `grep -n "wpt\|bench" justfile` returns
  nothing beyond the pre-existing `smoke` recipe's unrelated dependency on
  `benchmarks/harness/runner.test.mjs`.
- `security/supply-chain-manifest.json` has no WPT/html5lib-tests vendoring
  entries (not independently re-checked byte-for-byte in this pass, but
  consistent with no vendored directory existing to register).

Every single one of the plan's "Summary of concrete deliverables for the
implementer" (§ of the plan doc) is unbuilt. This matches what every merged
M2 crate's own evidence doc already discloses explicitly and consistently
(T03, T04, T05, T10, T13 all independently flag "no WPT/html5lib-tests
infrastructure exists anywhere in this repository" as a real, tracked gap,
each recommending a dedicated infrastructure task). **T14 cannot assume a
runnable WPT harness exists when it starts.** Either:

(a) a dedicated infrastructure task (Lane A + Lane B vendoring,
`crates/wpt-support`, the `wpt-subset` feature, `scripts/wpt/check-selection.mjs`)
lands as its own task before or alongside T06–T12, or
(b) T14 absorbs building the minimum slice of it needed to assemble "the
milestone target fixtures" corpus its own fast gate requires, using
hand-authored fixtures in the same style every M2 crate so far has used
(pre-authorized explicitly by each of those tasks' own fast-gate text, per
their evidence docs).

Recommend (a): five prior tasks (T03/T04/T05/T10/T13) have each
independently recommended a dedicated WPT-infrastructure task and none has
landed after eight merged M2 tasks — deferring it into T14 (already the
single highest-risk, no-parallel, "everything must land first" task) adds
scope to an already-loaded gate rather than parallelizing it against
T06–T09/T12's implementation work.

## 5. Benchmark methodology — what is actually benchmarkable, what is not

**Nothing is benchmarkable yet at the native-engine level, and no numbers
are reported here.**

- The plan's own benchmark target is **M2-T07** ("cold-vs-snapshot" V8
  startup) — T07 has not started (blocked on T06). There is no V8 bridge,
  no isolate, nothing to time.
- `criterion` is not a dependency anywhere in the workspace (`grep -n
  "criterion" Cargo.toml crates/*/Cargo.toml` returns nothing) and there is
  no `[workspace.dependencies]` table in root `Cargo.toml` to pin it into,
  consistent with the plan doc's own fact #2. No `[[bench]]` target exists
  in any crate.
- `scripts/bench/export-criterion.mjs` (the plan's criterion→existing-schema
  bridge script) does not exist.
- `benchmarks/reports/baselines/` does not exist (checked; only
  `benchmarks/{corpus,harness,reports}` and the top-level
  `reproducibility.mjs`/`runner.mjs`/`runner.test.mjs`/`smoke.mjs` files are
  present).
- **What *is* real and runnable today**: `benchmarks/harness/runner.mjs`
  (the JS/protocol-level workload harness — `runWorkload()`, schema
  `benchmarks/harness/runner.mjs`'s `BENCHMARK_SCHEMA_VERSION`, CPU/memory/
  attempts/verified fields) and its smoke test. Ran it directly in this pass:

```text
$ node benchmarks/harness/smoke.mjs
benchmark smoke: passed
```

  This harness is real and already used elsewhere in the repo (M0-T10), but
  per the plan's own analysis it "targets end-to-end protocol-level
  workloads, not Rust-internal micro-benchmarks" — it is the wrong tool for
  a native-engine cold-vs-snapshot or navigate-fixture-latency claim, though
  it is the *right* tool if M2-T14 ends up wanting to benchmark the native
  worker through the same command-bus surface Chromium is benchmarked
  through (an apples-to-apples native-vs-Chromium comparison at the
  protocol layer, which is closer to what T14's own "native/Chromium traces
  share the canonical outcome format" criterion implies than a Rust
  micro-benchmark would be).

**Concrete methodology for whatever performance claim M2-T14 needs to
make**, once T06–T09/T12 exist:

1. **If the claim is "native fixture X completes navigate→JS→query→close in
   N ms"**: reuse `benchmarks/harness/runWorkload()` exactly as-is (no new
   tooling needed) — define a `workload` that drives the real command bus
   against `NativeEngine` the same way `m1-compatibility-smoke.mjs` drives
   it against the injected control plane today, with a `verify()` step that
   checks the actual postcondition (document reached `load`, extraction
   returned the expected node), not just "didn't throw." Record the full
   `runWorkload()` output (`schema_version`, `build_id`, `environment_id`,
   attempts, CPU, memory) into `benchmarks/reports/`, per the harness's
   existing schema and sink — do not invent a second reporting shape.
2. **If the claim is "native is Nx faster than Chromium for fixture X"**:
   run the *same* workload definition against both engines through the same
   harness and the same fixture, report both raw distributions side by
   side, and follow `PERFORMANCE_BENCHMARKS.md`'s standing rule (referenced
   by the wave-1 plan) — report distributions, not a single number; verify
   both arms reached an equivalent, genuinely-completed postcondition inside
   `verify()`, not just wall time; never publish a bare multiplier without
   that evidence attached.
3. **If the claim is V8-internal (cold vs. snapshot startup, T07's own
   metric)**: that is T07's fast gate, not T14's — T07 already needs
   `criterion` wired in per the wave-1 plan's §4/§5, and its own evidence
   doc should carry the raw sample data. T14 should *read* T07's recorded
   `benchmarks/reports/baselines/v8-startup.json` (once T07 produces it)
   rather than re-deriving or re-measuring it.
4. **No number should be published in T14's own evidence doc unless it was
   produced by a command actually run in that task's sandbox**, per this
   repo's own evidence-over-narration culture (visible throughout every
   cited M2-T0x evidence doc's "Fast gate — commands and real output"
   sections) — if the sandbox available at T14's claim time cannot run a
   real V8 build (e.g. no toolchain, no host GPU/CPU parity with a
   reference host), T14 must record that as a named, dated gap in
   `agents/BLOCKERS.md`, not fabricate or extrapolate a number.

## 6. What T14 can pre-stage today (no code, no crate touched)

Not built in this pass (out of scope: "do not write implementation code, do
not modify any crate"), but named here so the claiming agent does not need
to re-derive it:

- A milestone-target-fixture list should be drafted before T06–T09/T12
  finish, so T14 can start assembling its corpus runner the moment they
  land rather than scoping fixtures from scratch. Candidates already implied
  by existing fixture infrastructure: `scripts/test/fixture-server.mjs`'s
  `/navigation`, `/form`, redirect/compression/chunked routes (T02's
  fixtures already exercise most of these at the network layer).
- The capability-declaration surface T14 needs to update
  (`protocols/CAPABILITY_MATRIX.md`, referenced by the wave-1 plan's "Files
  reviewed" list) should be diffed against the T06–T13 evidence docs' actual
  shipped surfaces once each lands, rather than re-reading each crate from
  scratch at T14 claim time — this readiness doc's §1/§2 tables are that
  starting point.
- `scripts/test/m1-compatibility-smoke.mjs` is the concrete style reference
  for the "M2 corpus suite" fast-gate item: a real fixture server, real
  command objects, explicit assertions on both success and failure paths
  (`COMMAND_CANCELLED`, `ELEMENT_NOT_FOUND`, `WORKER_LOST`), and an explicit
  `runtime_claim` field disclosing exactly what is and isn't really
  exercised (`"injected Chromium contract only; real process/container
  runtime unavailable under owner waiver"`). T14's own corpus runner should
  carry an equivalently honest `runtime_claim`/scope disclosure once native
  execution is real, rather than silently implying full native-Chromium
  parity.

## Files reviewed (this pass)

`planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md`,
`planning/MILESTONE_02_NATIVE_ENGINE.md` (alias),
`.agent-state/design/M2-WPT-and-benchmark-harness-plan.md`,
`.agent-state/evidence/M2-T01.md` through `M2-T05.md`, `M2-T10.md`,
`M2-T11.md`, `M2-T13.md`, `agents/WORK_QUEUE.md`, `agents/CURRENT_STATE.md`,
`scripts/test/m1-compatibility-smoke.mjs`, `benchmarks/harness/runner.mjs`,
`benchmarks/harness/smoke.mjs` (executed), `justfile`, root `Cargo.toml`,
directory listings of `tests/`, `crates/`, `benchmarks/`.
