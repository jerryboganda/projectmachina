---
title: "Autonomous Work Queue"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Show ready, active, blocked, and queued work while the master task graph remains the authoritative plan."
---
# Autonomous Work Queue

## Scheduling policy

Select critical-path and unblocking tasks first. Two tasks may be active only when their file scopes and shared contracts do not overlap.

## Ready

M2 native-engine source track is unblocked early under
`M2-ENTRY-WAIVER-M1-EXIT` (see `agents/WAIVERS.md`) while `BLK-003`
(Chromium-track only) remains open separately.

Real dependency graph (wave = earliest point a task can honestly start;
concurrency capped at two active builder/implementation agents per the
scheduling policy above):

| Rank | Task | Wave | Milestone | Role | Dependencies | Suggested lane |
| ---: | --- | ---: | --- | --- | --- | --- |
| — | ~~M2-T01~~ | 1 | M2 | native-engine | merged (#28) | — |
| 2 | M2-T02 | 2 | M2 | native-engine + security | M2-T01 (merged) | A |
| 3 | M2-T03 | 2 | M2 | native-engine | M2-T01 (merged) | B |
| — | ~~M2-T05~~ | 2 | M2 | native-engine | merged (#31) | — |
| 5 | M2-T06 | 2 | M2 | native-engine + security | M2-T01 (merged) | B |
| 6 | M2-T04 | 3 | M2 | native-engine | M2-T03, M2-T05 | A |
| 7 | M2-T07 | 3 | M2 | native-engine | M2-T06, M2-T05 | B |
| 8 | M2-T11 | 3 | M2 | native-engine | M2-T05 | A |
| 9 | M2-T08 | 4 | M2 | native-engine | M2-T06, M2-T07, M2-T02 | A |
| 10 | M2-T09 | 5 | M2 | native-engine | M2-T02, M2-T04, M2-T07, M2-T08 | A |
| 11 | M2-T10 | 5 | M2 | native-engine | M2-T05 | B |
| 12 | M2-T12 | 5 | M2 | native-engine | M2-T05, M2-T08 | A |
| 13 | M2-T13 | 6 | M2 | native-engine + agent-runtime | M2-T05, M2-T10 | A |
| 14 | M2-T14 | 7 | M2 | orchestrator + quality | M2-T01 through M2-T13, M1-T09 | A (no parallel) |

## Active

| Task | Owner | Branch/worktree | State | Heartbeat |
| --- | --- | --- | --- | --- |
| M2-T03 | wave2-builder-b | agent/M2-T03-* | claimed | — |
| M2-T02 | wave3-builder-a | agent/M2-T02-* | claimed | — |

M2-T05 merged (#31), unlocking M2-T10 and M2-T11 (both depend only on T05) —
next pair to start the moment a slot frees, per the 2-concurrent cap.

M2-T06 held back: no V8 build toolchain (`gn`/`gclient`/depot_tools) is
available locally; owner directed GitHub Actions ONLY for the V8 build
(`.github/workflows/v8-toolchain-build.yml`, workflow_dispatch, run in
progress) rather than local/VPS. M2-T06 proper (the C++ bridge/Rust facade
code) is blocked on that workflow producing real artifacts.

## In review

M0-T01 through M0-T12 and M1-T01/M1-T12 have merged hosted-gate/source evidence.
The injected M1 smoke passes; M1 exit remains blocked by BLK-003 pending real
Chromium-track runtime/listener evidence. M2 source track proceeds separately
under the recorded waiver.

## Blocked

BLK-003 — real Chromium process launch/CDP code does not exist in
`crates/chromium-adapter`; blocks M1 exit and any Chromium-track (non-native)
production claim. Does not block M2 native-engine source work — see
`agents/BLOCKERS.md` and `agents/M1_EXIT_REPORT.md`.

## Deferred until dependencies

See `planning/DEPENDENCY_MAP.md` and the milestone files. The orchestrator regenerates this view as tasks advance.

## Queue mutation rules

- Do not change task IDs or acceptance criteria here; change the milestone source.
- Add status, owner, branch, and evidence links only.
- Every active item needs a current claim and heartbeat.
- Every blocked item needs reproduction evidence and a next decision/action.
