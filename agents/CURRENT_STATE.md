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

- Status: `M0_EXIT_CANDIDATE_BLOCKED`
- Active milestone: `M0 — Foundation and governance`
- Current release target: `0.1.0-alpha`
- Last reconciled: `2026-08-09`
- Default branch: `main` (M0-T01 through M0-T12 source work merged)

## Active tasks

| Task | Owner | Branch/worktree | State | Heartbeat | Blocker |
| --- | --- | --- | --- | --- | --- |
| None | — | — | — | — | — |

## Recently completed

- Agentic development documentation pack generated.
- Recommended architecture and testing policy established.
- M0-T01 bootstrap, M0-T02 shared claim/worktree/evidence tooling, M0-T03 protected
  fast-gate policy, M0-T04 command contract validation, and M0-T05 architecture
  boundary fixtures/reporting, M0-T06 security baseline, M0-T07 supply-chain
  provenance controls, M0-T08 deterministic fixtures, M0-T09 telemetry/evidence
  primitives, M0-T10 reproducible benchmark smoke, M0-T11 local stack health
  controls, and M0-T12 real two-worktree rehearsal are merged.
- Runtime foundation continuation adds capability snapshots, the single command bus, explicit fallback metadata, and bounded session/page resource accounting.

## Next ready tasks

1. Install Docker/Compose and complete local-stack health/reset evidence for M0 exit.

## Human gates pending

- None required to continue local development with recommended defaults.

## Known blockers

- M0 fast-gate source checks pass locally with Node 20.18.0, Rust 1.86.0, CMake 4.4.2, Ninja 1.13.2, Buf 1.47.2, and pnpm 9.15.0. Docker Desktop provisioning failed at the required administrator/UAC step.

## Reconciliation notes

M0-T01 through M0-T12 have merged hosted-gate evidence and source evidence. M0 exit
remains blocked only by Docker/Compose health and reset rehearsal on this host.
