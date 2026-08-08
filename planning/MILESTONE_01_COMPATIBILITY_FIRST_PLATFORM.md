---
title: "M1 — Compatibility-First Platform"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Program and Architecture"
purpose: "Provide executable task packets and exit criteria for milestone M1."
---

# M1 — Compatibility-First Platform

## Objective

Deliver a useful Chromium-backed service behind the final command, policy, scheduling, event, SDK and observability foundations so the product works while native capability is built.

## Entry criteria

- M0 exit approved.
- Canonical schema and local infrastructure available.

## Exit criteria

- A verified real browser task completes through supported initial APIs/SDKs.
- Session lifecycle, routing, worker isolation, events, traces and errors are production-shaped.
- Native engine can plug into the same contract without redesign.

## Scheduling notes

The orchestrator may run at most two implementation tasks concurrently. Parallel tasks must have non-overlapping write scopes and no unmerged shared-contract dependency. Every task follows `agents/AUTONOMOUS_LOOP.md` and records deferred heavy validation.

## Tasks

## M1-T01 — Implement control-plane database schema and durable event outbox

**Primary role:** platform  
**Dependencies:** M0-T01, M0-T04, M0-T09  
**Risk:** critical  
**May run in parallel:** constrained

### Deliverables

- Create PostgreSQL migrations for organizations, projects, policies, sessions, workers, artifacts, workflows, usage and audit metadata.
- Implement transactional outbox and optimistic version fields.
- Add repository/service interfaces without worker direct database access.

### Acceptance criteria

- Migrations apply, rollback/forward policy is documented, and clean schema matches model.
- Outbox event and aggregate update commit atomically.
- Tenant-scoped queries include authorization context.

### Fast gate

- Run migration up/down/compatibility smoke.
- Test transaction rollback and duplicate event idempotency.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M1-T02 — Implement authentication, tenancy, authorization and project policy

**Primary role:** security + platform  
**Dependencies:** M1-T01, M0-T06  
**Risk:** critical  
**May run in parallel:** constrained

### Deliverables

- Add local/service authentication abstraction and project API credential flow.
- Implement server-side organization/project/resource authorization and policy resolution.
- Add audit records for credential and policy operations.

### Acceptance criteria

- Cross-tenant access is denied for resource and event queries.
- Credentials can be scoped, rotated and revoked.
- Effective policy hash is stable and attached to session requests.

### Fast gate

- Run positive/negative/cross-tenant tests.
- Run secret/log redaction check for credential operations.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M1-T03 — Implement session lifecycle service and idempotent control API

**Primary role:** platform  
**Dependencies:** M1-T01, M1-T02, M0-T04  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Implement create/get/cancel/close session state machine and idempotency keys.
- Resolve immutable effective policy and persist lifecycle transitions.
- Expose internal service interface and initial HTTP endpoints.

### Acceptance criteria

- Repeated create with same key returns same normalized session.
- Invalid transitions fail with canonical codes.
- Cancellation/close are idempotent and terminal state persists.

### Fast gate

- Run state-machine/property tests.
- Run HTTP create/cancel/close smoke.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M1-T04 — Implement scheduler, worker registry, leases and fair queue

**Primary role:** platform  
**Dependencies:** M1-T01, M1-T03  
**Risk:** critical  
**May run in parallel:** constrained

### Deliverables

- Register worker classes/versions/capabilities and health.
- Implement bounded tenant-aware queue, placement filter/scoring and session leases.
- Handle heartbeat, suspect/lost, drain and recycle states.

### Acceptance criteria

- A session is placed only on compatible engine/isolation/region worker.
- Stale leases resolve without duplicate unsafe assignment.
- Tenant quotas/backpressure return typed outcomes.

### Fast gate

- Run placement and lease property tests.
- Inject worker loss during queued/starting session.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M1-T05 — Build prewarmed Chromium worker pool and isolation controls

**Primary role:** platform + security  
**Dependencies:** M1-T04, M0-T11  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Launch pinned Chromium in shared-development and dedicated-process worker modes.
- Implement context/process limits, drain, recycle, health and crash classification.
- Apply filesystem/network/resource safe defaults.

### Acceptance criteria

- Worker advertises exact Chromium/version/capabilities.
- Crash affects only expected isolation boundary and is reported.
- No host secrets or unrestricted filesystem/network are available.

### Fast gate

- Run context/session lifecycle and crash injection.
- Run baseline sandbox/config negative checks.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M1-T06 — Implement Chromium canonical engine adapter

**Primary role:** protocol + platform  
**Dependencies:** M1-T05, M0-T04  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Translate core session/page/navigation/evaluate/query/interaction/cookie commands to Chromium.
- Translate lifecycle, console, network and errors to canonical events/outcomes.
- Return engine/version/resource and verification metadata.

### Acceptance criteria

- A command through adapter behaves consistently with canonical fixtures.
- Unsupported adapter command fails explicitly.
- Cancellation and deadline propagate to browser operations.

### Fast gate

- Run canonical engine contract suite against Chromium.
- Run timeout/cancel/unsupported negative tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M1-T07 — Expose initial HTTP, gRPC and event-stream adapters

**Primary role:** protocol  
**Dependencies:** M1-T03, M1-T06  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Implement HTTP command endpoint and SSE event stream.
- Implement gRPC unary execute and basic bidirectional stream.
- Map canonical errors, deadlines, auth and correlation.

### Acceptance criteria

- Same sample task has equivalent canonical outcome through HTTP and gRPC.
- Events are ordered and resumable within buffer.
- Unauthorized stream/resource access is denied.

### Fast gate

- Run transport contract and reconnect smoke.
- Run malformed/oversized request negative tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M1-T08 — Implement ordered events, idempotent delivery and backpressure

**Primary role:** platform + protocol  
**Dependencies:** M1-T01, M1-T07  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Persist/project critical lifecycle events and implement per-session sequence.
- Add bounded subscriber buffers, acknowledgement/resume and resync-required behavior.
- Make event consumers idempotent.

### Acceptance criteria

- Events preserve session order across reconnect.
- Slow consumers cannot cause unbounded memory.
- Outbox replay does not duplicate durable effects.

### Fast gate

- Run reconnect/gap/slow-reader tests.
- Run duplicate delivery property test.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M1-T09 — Implement capability registry and router version zero

**Primary role:** architect + platform  
**Dependencies:** M0-T04, M1-T04, M1-T06  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Store versioned capability snapshots and policy statuses.
- Implement `chromium-only`, `prefer-compatible` and placeholder `prefer-native` routing.
- Emit structured decision record and capability API.

### Acceptance criteria

- Router never assigns unsupported worker.
- Decision includes reason, version and policy.
- Capability response and runtime adapter evidence agree.

### Fast gate

- Run deterministic routing table tests.
- Test disabled-by-policy and no-capacity outcomes.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M1-T10 — Implement traces, classified artifacts and reproduction bundle version zero

**Primary role:** observability + platform  
**Dependencies:** M0-T09, M1-T06, M1-T08  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Instrument API, scheduling, Chromium commands, lifecycle and errors.
- Store encrypted/classified artifact metadata and object data.
- Generate redacted reproduction bundle for fixture failure.

### Acceptance criteria

- A session trace spans client request to worker outcome.
- Bundle validates hashes and contains no canary secret.
- Artifact access is tenant authorized and signed URL expires.

### Fast gate

- Run failure-bundle end-to-end smoke.
- Run cross-tenant artifact and canary tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M1-T11 — Publish TypeScript and Python alpha SDKs

**Primary role:** protocol  
**Dependencies:** M1-T07, M1-T08  
**Risk:** medium  
**May run in parallel:** yes

### Deliverables

- Generate models/low-level clients and add ergonomic async session/page facade.
- Implement typed errors, deadlines, cancellation, events and cleanup.
- Add clean-environment quick starts.

### Acceptance criteria

- Both SDKs create a session, navigate fixture, extract and close.
- Canonical errors are typed without string parsing.
- Event reconnect and resource cleanup work.

### Fast gate

- Run SDK quick starts in clean language environments.
- Run server/client compatibility contract tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M1-T12 — Deliver compatibility-first end-to-end platform slice

**Primary role:** orchestrator + quality  
**Dependencies:** M1-T01 through M1-T11  
**Risk:** critical  
**May run in parallel:** no

### Deliverables

- Integrate API, policy, scheduler, Chromium worker, events, SDK and trace console placeholder.
- Run representative navigation/form/extraction task through HTTP, gRPC, TypeScript and Python.
- Produce M1 exit report and native-engine ready interfaces.

### Acceptance criteria

- Verified task succeeds through all selected clients with one canonical trace.
- Failure/cancel/unsupported paths are explicit and reproducible.
- Control and data-plane restart behavior is classified and state reconciles.

### Fast gate

- Run full M1 e2e smoke matrix.
- Inject client disconnect and worker crash.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.
