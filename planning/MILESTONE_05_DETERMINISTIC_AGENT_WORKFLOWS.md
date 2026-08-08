---
title: "M5 — Deterministic Agent and Workflow Runtime"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Program and Architecture"
purpose: "Provide executable task packets and exit criteria for milestone M5."
---

# M5 — Deterministic Agent and Workflow Runtime

## Objective

Turn successful agent/browser interactions into typed, versioned, replayable workflows with stable semantic locators, schema extraction, secrets, approvals and optional bounded recovery.

## Entry criteria

- M4 command/protocol surfaces and M3 action/state capabilities are available.

## Exit criteria

- Normal workflow execution needs no LLM.
- Restart, migration and side effects are checkpoint safe.
- Secrets and high-impact actions follow enforceable policy and audit.

## Scheduling notes

The orchestrator may run at most two implementation tasks concurrently. Parallel tasks must have non-overlapping write scopes and no unmerged shared-contract dependency. Every task follows `agents/AUTONOMOUS_LOOP.md` and records deferred heavy validation.

## Tasks

## M5-T01 — Define the versioned deterministic workflow DSL and schema

**Primary role:** agent-runtime + architect  
**Dependencies:** M4-T01, M4-T10  
**Risk:** critical  
**May run in parallel:** constrained

### Deliverables

- Define workflow metadata, inputs, secret references, steps, locators, waits, branches/retries, checkpoints, outputs and approvals.
- Create JSON Schema and human-readable TypeScript-like authoring representation.
- Define versioning, validation, capability requirements and prohibited arbitrary host execution.

### Acceptance criteria

- Representative login/extract/form workflow validates and round trips.
- Invalid side effect without policy/checkpoint is rejected.
- Schema is engine/protocol neutral and migration versioned.

### Fast gate

- Run schema/property/round-trip tests.
- Run hostile expression/oversized workflow negative tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M5-T02 — Build the session recorder and normalized action log

**Primary role:** agent-runtime  
**Dependencies:** M3-T14, M4-T10, M5-T01  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Record navigation, locators, verified actions, waits, outputs and checkpoints from successful sessions.
- Replace secret values with references and normalize volatile IDs.
- Capture capability/engine/fallback context and replay safety class.

### Acceptance criteria

- Recording of a sample session compiles without secret/page-sensitive leakage beyond policy.
- Equivalent repeated observations are compacted without changing semantics.
- Unverified side-effect boundaries are visible.

### Fast gate

- Run recorder fixture and canary secret scan.
- Compare recorded steps to canonical event timeline.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M5-T03 — Implement workflow compiler and static analyzer

**Primary role:** agent-runtime  
**Dependencies:** M5-T01, M5-T02  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Compile recording/DSL to executable typed plan and capability manifest.
- Analyze undefined variables, locator ambiguity risk, unreachable steps, unsafe retries/side effects and approval needs.
- Produce deterministic plan hash and diagnostics.

### Acceptance criteria

- Same source/version produces same plan hash.
- Unsafe or unsupported plan is rejected before execution when detectable.
- Diagnostics link exact step and remediation.

### Fast gate

- Run golden compiler/analyzer suite.
- Run mutation/property tests for invalid plans.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M5-T04 — Implement deterministic executor, checkpoints, pause and resume

**Primary role:** agent-runtime  
**Dependencies:** M5-T03, M4-T01, M3-T14  
**Risk:** critical  
**May run in parallel:** constrained

### Deliverables

- Execute compiled steps through canonical commands without an LLM.
- Persist run/step state, inputs/outputs, verified checkpoints and idempotency.
- Support cancellation, pause/resume, engine migration and terminal classification.

### Acceptance criteria

- Normal sample workflow replays successfully with zero LLM calls.
- Restart resumes from last safe checkpoint without repeating unsafe action.
- Every step has exact outcome, revisions and trace link.

### Fast gate

- Run restart/migration/side-effect checkpoint suite.
- Run cancellation and duplicate-delivery tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M5-T05 — Implement stable semantic locators and repair candidates

**Primary role:** agent-runtime + native-engine  
**Dependencies:** M3-T13, M5-T03  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Define locators using role/name/state/label/text/test ID and bounded structural hints.
- Score candidates across revisions and return ambiguity rather than guessing beyond threshold.
- Store repair suggestions separately from automatically accepted workflow changes.

### Acceptance criteria

- Minor fixture DOM changes still resolve stable intended targets.
- Ambiguous/adversarial page fails safely.
- Locator evidence includes why candidate matched and revision.

### Fast gate

- Run locator mutation corpus and ambiguity tests.
- Benchmark semantic delta-based resolution.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M5-T06 — Implement schema-guided extraction and validation

**Primary role:** agent-runtime + native-engine  
**Dependencies:** M3-T13, M5-T03  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Define output schemas, field locators/transforms, cardinality, validation and provenance.
- Extract from live DOM/semantic indexes in one pass where possible.
- Return field-level errors/confidence/source revisions and bounded samples.

### Acceptance criteria

- Representative invoice/list/profile schemas validate with expected provenance.
- Missing/ambiguous/invalid fields are explicit.
- Extraction cannot execute arbitrary host code or leak unrelated secrets.

### Fast gate

- Run schema extraction golden/differential tests.
- Run large-output and hostile transform tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M5-T07 — Implement secret references, action policy and approval service

**Primary role:** agent-runtime + security  
**Dependencies:** M5-T04, M1-T02  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Integrate vault abstraction, scoped execution grants and moment-of-use secret resolution.
- Implement high-impact action classification and approval policies/records.
- Expose API/events for approval request, decision, expiry and audit.

### Acceptance criteria

- Secret never appears in workflow, trace, recording, error or console response.
- Required approval pauses before action and binds exact version/run/step/destination.
- Expired/denied approval cannot be reused.

### Fast gate

- Run canary secret end-to-end and approval race tests.
- Run page prompt-injection attempt against policy.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M5-T08 — Implement bounded recovery and selector repair using optional AI

**Primary role:** agent-runtime + security  
**Dependencies:** M5-T05, M5-T07  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Define recovery trigger, context packet, model/tool permissions, token/time/attempt budgets and output schema.
- Allow observe/repair proposal under fixed policy; require verification and workflow version update approval as configured.
- Record model/version/cost/decision without sensitive page data beyond policy.

### Acceptance criteria

- Recovery never expands egress, secret or action permission.
- Successful repair is verified before run continues.
- Budget exhaustion returns classified failure and deterministic handoff.

### Fast gate

- Run controlled locator-break recovery fixtures.
- Run prompt-injection/tool-escalation and budget tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M5-T09 — Implement workflow versions, schedules, runs and retention service

**Primary role:** platform + agent-runtime  
**Dependencies:** M1-T01, M5-T04, M5-T07  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Persist immutable workflow versions, activation/rollback, schedules, runs, steps, approvals and outputs.
- Implement scheduler leases/idempotency, tenant quotas and retention.
- Expose HTTP/gRPC/SDK lifecycle and audit.

### Acceptance criteria

- Concurrent scheduler delivery creates one logical run.
- Rollback selects exact prior approved version.
- Deletion/retention removes outputs/artifacts according to policy.

### Fast gate

- Run schedule/idempotency/version/rollback tests.
- Run tenant authorization and retention tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M5-T10 — Certify deterministic workflow corpus and milestone exit

**Primary role:** quality + orchestrator  
**Dependencies:** M5-T01 through M5-T09  
**Risk:** critical  
**May run in parallel:** no

### Deliverables

- Create corpus for authentication simulation, extraction, forms, multi-page, migration, approval and recoverable locator drift.
- Measure replay success, zero-LLM normal runs, recovery cost and side-effect safety.
- Publish limitations and deferred final tests.

### Acceptance criteria

- Target workflows pass repeated deterministic runs.
- No normal replay invokes an LLM.
- No secret leak, unsafe repeat or unapproved high-impact action occurs.

### Fast gate

- Run workflow corpus with restart/migration/failure injection.
- Run canary secret and approval audit suite.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.
