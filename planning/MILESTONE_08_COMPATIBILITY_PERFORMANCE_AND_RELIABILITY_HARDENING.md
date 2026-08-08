---
title: "M8 — Compatibility, Performance, and Reliability Hardening"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Program and Architecture"
purpose: "Provide executable task packets and exit criteria for milestone M8."
---

# M8 — Compatibility, Performance, and Reliability Hardening

## Objective

Use beta evidence and broad qualification suites to fix standards, protocol, security, performance, routing, cross-platform and reliability gaps before freezing a release candidate.

## Entry criteria

- Controlled beta evidence is available.
- All major product surfaces are feature complete for release scope.

## Exit criteria

- Broad WPT/differential/conformance/security/fuzz/soak qualification supports release.
- Performance and fallback economics are measured and improved without lowering correctness.
- An immutable evidence-backed release candidate is approved for M9.

## Scheduling notes

The orchestrator may run at most two implementation tasks concurrently. Parallel tasks must have non-overlapping write scopes and no unmerged shared-contract dependency. Every task follows `agents/AUTONOMOUS_LOOP.md` and records deferred heavy validation.

## Tasks

## M8-T01 — Expand and triage prioritized Web Platform Tests

**Primary role:** quality + native-engine  
**Dependencies:** M3-T15, M4-T09  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Run broad P0/P1 WPT shards for implemented HTML/DOM/navigation/events/forms/fetch/cookies/storage/frames/workers/WebSockets.
- Triage failures/timeouts/crashes to defect, expected limitation or harness issue with capability link.
- Fix highest-impact semantic clusters and maintain expected-results metadata.

### Acceptance criteria

- No unexplained crash/timeout cluster in enabled capability.
- Expected failures are specific, owned and versioned.
- Priority pass/regression trend supports RC criteria.

### Fast gate

- Run broad WPT qualification shards.
- Rerun fixed clusters and focused regressions.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M8-T02 — Expand deterministic and approved real-workload differential corpus

**Primary role:** quality + native-engine  
**Dependencies:** M3-T15, M7-T12  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Add fixtures and privacy-approved beta reproductions for high-value divergences.
- Compare native/Chromium lifecycle, DOM/semantic, network/storage and verified actions.
- Improve normalizers/oracles and fix high-impact differences.

### Acceptance criteria

- Corpus is versioned, reproducible and legally/privacy reviewed.
- Divergences have classification and capability/routing outcome.
- No known native false-success on target corpus.

### Fast gate

- Run broad differential corpus in native-only/hybrid.
- Run nondeterminism/repeatability analysis.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M8-T03 — Harden full protocol and ecosystem client matrices

**Primary role:** protocol + quality  
**Dependencies:** M4-T06, M4-T09, M4-T10, M4-T12  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Run expanded HTTP/gRPC, Playwright/Puppeteer, Selenium/BiDi, MCP and SDK version matrices.
- Fix lifecycle/event/error/backpressure/disconnect and unsupported gaps.
- Generate public-compatible evidence/limitation matrix.

### Acceptance criteria

- All claimed tuples pass required journeys and negative cases.
- No silent unsupported or indefinite hang in certified surfaces.
- Version/deprecation behavior is verified.

### Fast gate

- Run broad conformance matrix.
- Run malformed/slow/disconnect stress suites.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M8-T04 — Run sustained parser, DOM, protocol and V8 bridge fuzzing/sanitizers

**Primary role:** quality + security + native-engine  
**Dependencies:** M2-T03, M2-T05, M2-T06, M4-T01  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Expand fuzz corpora/targets and run hours-scale sanitizer campaigns.
- Minimize, triage and fix crashes, leaks, UAF/races/UB and unbounded behavior.
- Add regressions and security handling for sensitive findings.

### Acceptance criteria

- No unresolved critical/high reachable memory-safety or parser crash.
- All findings have minimized input and disposition.
- Sanitizer builds complete representative engine/worker lifecycle.

### Fast gate

- Run scheduled hours-scale campaigns and sanitizer integration.
- Rerun minimized regression corpus.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M8-T05 — Harden tenant, sandbox, egress, secrets and workflow security suites

**Primary role:** security + quality  
**Dependencies:** M7-T01 through M7-T07  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Run comprehensive cross-tenant, sandbox, SSRF/rebinding, proxy, artifact, secret-canary, prompt-injection and approval tests.
- Add independent security review/penetration preparation and fix findings.
- Validate emergency controls and audit completeness.

### Acceptance criteria

- No critical/high unauthorized access, egress, secret leak or unsafe action.
- Isolation tier claims match actual configuration/tests.
- Security evidence is release-candidate ready.

### Fast gate

- Run broad security suite on staging-like topology.
- Run canary secret scan across all stores/artifacts.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M8-T06 — Optimize startup snapshots, memory, scheduling and native effective throughput

**Primary role:** performance + native-engine  
**Dependencies:** M2-T07, M3-T15, M7-T09  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Profile cold/warm startup, V8/DOM/network memory, event queues, allocators and worker density.
- Implement measured snapshot/cache/arena/scheduler/recycle improvements without semantic shortcuts.
- Add regression budgets and resource dashboards.

### Acceptance criteria

- Material improvements are shown on verified tasks with equal fidelity.
- No success/compatibility/security regression hides performance gain.
- Memory stabilizes in qualification soak.

### Fast gate

- Run controlled benchmark and profiler suite.
- Run affected WPT/differential/resource regression.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M8-T07 — Improve routing prediction, migration success and fallback economics

**Primary role:** architect + performance + platform  
**Dependencies:** M3-T14, M7-T12, M8-T02  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Analyze fallback by capability/domain/workload/version and false native/Chromium choices.
- Improve deterministic pre-routing and migration checkpoints; add privacy-safe domain hints if justified.
- Implement cost/latency policy and circuit breakers.

### Acceptance criteria

- Hybrid verified success remains at/above target while unnecessary fallback decreases.
- Migration never weakens policy or repeats unsafe action.
- Every prediction/decision remains explainable and versioned.

### Fast gate

- Run router replay simulation on beta/corpus data.
- Run migration failure and side-effect safety suite.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M8-T08 — Execute qualification load, soak, chaos and recovery testing

**Primary role:** quality + platform  
**Dependencies:** M7-T09, M7-T11, M8-T06, M8-T07  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Run concurrency sweeps, 4–8 hour mixed soak and worker/control/data fault injection.
- Measure queue fairness, memory/FD/thread growth, crash containment, leases, retries and recovery.
- Fix release-blocking reliability defects and update runbooks.

### Acceptance criteria

- No unbounded growth, deadlock, cross-tenant effect, unsafe replay or durable-state corruption.
- Autoscaling/backpressure/recycle/rollback behave within thresholds.
- Crash-free and tail latency trend support RC.

### Fast gate

- Run qualification soak and chaos plan.
- Rerun focused recovered defects.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M8-T09 — Complete supported platform and architecture builds

**Primary role:** platform + native-engine  
**Dependencies:** M7-T08, M8-T04  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Build/test Linux x86_64/arm64, macOS targets and native Windows target according to release scope.
- Resolve toolchain/V8/Chromium/sandbox/package differences and document limitations.
- Create clean install/package smoke for each supported target.

### Acceptance criteria

- Release-scope targets build reproducibly and quick starts pass.
- Unsupported platform features fail at install/start with clear message.
- No target silently weakens security defaults.

### Fast gate

- Run clean cross-platform CI/build/package matrix.
- Run native/Chromium fixture smoke per target.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M8-T10 — Verify API, schema, workflow and data backward compatibility

**Primary role:** architect + protocol + platform  
**Dependencies:** M4-T01, M5-T09, M7-T10  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Test supported previous SDK/server combinations, protobuf/OpenAPI changes, workflow versions and state bundles.
- Test database expand/migrate/contract and rollback windows.
- Fix or document migrations/deprecations.

### Acceptance criteria

- Supported stored workflows and clients continue or receive documented migration.
- No destructive data change lacks backup/rollback path.
- Capability/version response is reproducible for old/new builds.

### Fast gate

- Run compatibility and migration matrix.
- Run rollback/forward recovery smoke.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M8-T11 — Generate documentation, capability, evidence and release-candidate reports

**Primary role:** developer-experience + release  
**Dependencies:** M8-T01 through M8-T10  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Generate capability/client/error/API/SDK docs from tested sources.
- Assemble requirements-to-test evidence index and known limitations.
- Validate quick starts, examples, links and benchmark methodology draft.

### Acceptance criteria

- Published draft matches runtime registry and test artifacts.
- No claim lacks versioned evidence or limitation.
- All examples execute in clean target environments.

### Fast gate

- Run docs generation/link/example checks.
- Cross-check random capabilities against runtime/test registry.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M8-T12 — Conduct independent release-candidate readiness review

**Primary role:** release + independent reviewer  
**Dependencies:** M8-T01 through M8-T11  
**Risk:** critical  
**May run in parallel:** no

### Deliverables

- Review product scope, open risks, security, compatibility, performance, operations and deferred-test ledger.
- Select/freeze RC candidate or return precise blockers.
- Finalize M9 environment, versions, corpora, pass gates and owners.

### Acceptance criteria

- No unresolved blocker is hidden or reclassified without authority.
- RC artifacts are immutable, signed/hashed and reproducible.
- M9 can run without missing harness, data, credentials or acceptance definition.

### Fast gate

- Run RC readiness checklist and evidence audit.
- Rebuild/verify candidate artifact identity.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.
