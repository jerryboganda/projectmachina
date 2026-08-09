---
title: "Current Project State"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Provide the durable, human-readable snapshot that each autonomous cycle reconciles with repository facts."
---
# Current Project State

> This file is machine-maintained after repository bootstrap. Until then, it records the documentation baseline.

## Project

- Status: `M1_EXIT_BLOCKED_RUNTIME_M2_SOURCE_IN_PROGRESS`
- Active milestone: `M1 exit blocked (BLK-003, Chromium-track); M2 — Native engine fundamentals, source track underway under M2-ENTRY-WAIVER-M1-EXIT`
- Current release target: `0.1.0-alpha`
- Last reconciled: `2026-08-09`
- Default branch: `main` (M0 exit waiver recorded; M1-T01 through M1-T12 source work merged; M2-T01/T05 merged; boundary-checker tooling fix merged)

## Active tasks

| Task | Owner | Branch/worktree | State | Heartbeat | Blocker |
| --- | --- | --- | --- | --- | --- |
None.

## Recently completed

- Agentic development documentation pack generated.
- Recommended architecture and testing policy established.
- M0-T01 bootstrap, M0-T02 shared claim/worktree/evidence tooling, M0-T03 protected
  fast-gate policy, M0-T04 command contract validation, and M0-T05 architecture
  boundary fixtures/reporting, M0-T06 security baseline, M0-T07 supply-chain
  provenance controls, M0-T08 deterministic fixtures, M0-T09 telemetry/evidence
  primitives, M0-T10 reproducible benchmark smoke, M0-T11 local stack health
  controls, and M0-T12 real two-worktree rehearsal are merged.
- M1-T01 control-plane schema/outbox, M1-T02 scoped auth/policy primitives,
  M1-T03 idempotent session lifecycle, and M1-T04 fair scheduler/worker leases
  plus M1-T05 explicit worker pool/isolation contracts and M1-T06 Chromium adapter
  boundary, and M1-T07 initial HTTP/gRPC/event contracts are merged.
  M1-T08 now adds durable per-session event sequencing, bounded subscriber
  delivery, explicit resync recovery, and idempotent outbox projection.
  M1-T09 now adds versioned capability snapshots, policy-aware eligibility, and
  structured routing decisions with both-engine evidence.
  M1-T10 now adds bounded request-to-worker traces, scoped classified artifacts,
  signed expiry grants, and redacted hashed reproduction bundles.
  M1-T11 now adds publishable TypeScript/Python alpha SDKs with typed outcomes,
  deadlines, cancellation, reconnect, and cleanup. M1-T12 adds the honest
  injected compatibility smoke and exit blocker; it does not claim live runtime
  integration.
- M0 Docker/Compose runtime evidence is explicitly waived by owner option B;
  limitation remains recorded in `agents/WAIVERS.md`.
- Runtime foundation continuation adds capability snapshots, the single command bus, explicit fallback metadata, and bounded session/page resource accounting.
- `M2-ENTRY-WAIVER-M1-EXIT` recorded in `agents/WAIVERS.md`: M2 native-engine source
  work begins before M1 formally exits, since `BLK-003` is Chromium-track only.
  Nine wave-1 research/design/security agents produced implementation-ready specs
  for M2-T02/T03/T05/T06/T08 plus an M1/M2 contract compatibility checklist and a
  WPT/benchmark harness plan (`.agent-state/design/`, merged via #27).
- **M2-T01** (native engine session/context/page/resource accounting) is merged
  (#28): `ContextId`/`Context` identities, shared create/close/cancel lifecycle
  contract across `Session`/`Context`/`Page` with cascading cancellation, six
  bounded per-page resource categories with atomic checked-add hard limits, and
  an `EngineSession` facade composed symmetrically by `NativeEngine`/
  `ChromiumEngine`. `cargo test --workspace` and the architecture boundary check
  both passed. Context/page operations are exposed as direct Rust APIs only —
  no `CommandKind` was added (deferred to whichever task first needs bus-level
  context/page commands, per the M1/M2 contract checklist's recommendation).
- **M2-T05** (compact DOM nodes, handles, mutation, lifecycle) is merged
  (#31): `crates/dom`, arena/generational-handle model, two-phase mutation,
  structural teardown, zero `unsafe`, zero deps beyond std, 35/35 tests,
  clippy-clean across the workspace. Implemented live against an independent
  security review commissioned in parallel: `Generation` widened `u32`→`u64`
  after a real wraparound-aliasing finding; `Document::destroy_node` added
  after a real headline finding that no node-reclamation path existed for
  ordinary detach-and-abandon (would have caused unbounded memory growth);
  two self-aliased-argument link-corruption bugs fixed. Also incorporates
  M2-T04's coordination request (`create_element_ns`, `create_document_type`).
  Full disposition in `.agent-state/evidence/M2-T05.md`.
- Architecture boundary checker fixed (#30): now scans `Cargo.toml`
  dependency tables, catches the underscore-form Rust import that previously
  slipped through, and adds the native→protocol direction rule that was
  entirely unpoliced before.
- V8 toolchain provisioning moved to GitHub Actions per explicit owner
  direction — `.github/workflows/v8-toolchain-build.yml` (workflow_dispatch),
  not this local machine or the VPS. `M2-T06` proper (the C++ bridge/Rust
  facade code) is blocked on that workflow producing real, checksummed
  artifacts.

## Next ready tasks

1. `M2-T02` — native URL/DNS/TLS/HTTP streaming loader (in progress).
2. `M2-T03` — streaming HTML tokenizer (in progress).
3. `M2-T10` — CSS selector queries and initial XPath (design ready, unlocked by T05).
4. `M2-T11` — event dispatch, focus, basic input model (design ready, unlocked by T05).
5. `M2-T04` — HTML tree builder (design ready, waiting on T03).

Per the milestone scheduling policy, at most two of these run as concurrent
implementation agents at a time.

## Human gates pending

- None required to continue local development with recommended defaults.

## Known blockers

- M0 source/hosted gates pass; Docker/Compose health is waived for M1 and remains
  a pre-beta release limitation.

## Reconciliation notes

M0-T01 through M0-T12 and M1-T01/M1-T12 have merged hosted-gate/source evidence.
The injected M1 compatibility smoke passes, but `BLK-003` blocks real
Chromium/listener/SDK integration and therefore M1 exit. The owner Docker waiver
does not authorize a production/container readiness claim.
