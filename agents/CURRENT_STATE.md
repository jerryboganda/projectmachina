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

- Status: `M1_EXIT_BLOCKED_RUNTIME`
- Active milestone: `M1 — Compatibility-first platform`
- Current release target: `0.1.0-alpha`
- Last reconciled: `2026-08-09`
- Default branch: `main` (M0 exit waiver recorded; M1-T01 through M1-T12 source work merged)

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

## Next ready tasks

1. None — M1 exit is blocked by `BLK-003` until the real browser/listener runtime
   is provisioned and the M1 runtime gate is rerun.

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
