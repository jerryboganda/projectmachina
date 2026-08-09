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
| — | ~~M2-T02~~ | 2 | M2 | native-engine + security | merged (#36) | — |
| — | ~~M2-T03~~ | 2 | M2 | native-engine | merged (#34) | — |
| — | ~~M2-T05~~ | 2 | M2 | native-engine | merged (#31, independently re-verified — see note below) | — |
| 5 | M2-T06 | 2 | M2 | native-engine + security | M2-T01 (merged) | B |
| — | ~~M2-T04~~ | 3 | M2 | native-engine | merged (#38) | — |
| 7 | M2-T07 | 3 | M2 | native-engine | M2-T06, M2-T05 | B |
| 8 | M2-T11 | 3 | M2 | native-engine | M2-T05 (merged) | A |
| 9 | M2-T08 | 4 | M2 | native-engine | M2-T06, M2-T07, M2-T02 | A |
| 10 | M2-T09 | 5 | M2 | native-engine | M2-T02 (merged), M2-T04 (merged), M2-T07, M2-T08 | A |
| 11 | M2-T10 | 5 | M2 | native-engine | M2-T05 (merged) | B |
| 12 | M2-T12 | 5 | M2 | native-engine | M2-T05, M2-T08 | A |
| 13 | M2-T13 | 6 | M2 | native-engine + agent-runtime | M2-T05, M2-T10 | A |
| 14 | M2-T14 | 7 | M2 | orchestrator + quality | M2-T01 through M2-T13, M1-T09 | A (no parallel) |

## Active

| Task | Owner | Branch/worktree | State | Heartbeat |
| --- | --- | --- | --- | --- |
| M2-T11 | wave3-builder-c | agent/M2-T11-event-dispatch | claimed | — |
| M2-T10 | wave3-builder-d | agent/M2-T10-selectors | claimed | — |

**Process correction (2026-08-09):** the orchestrator briefly recorded
M2-T05 as merged before it actually was (PR #31 was still open, only its CI
had passed). This was caught before real damage: `crates/dom` was verified
missing from `main` via independent `git ls-tree`/`find` checks, PR #31 was
then genuinely merged (with a real Cargo.toml/Cargo.lock conflict resolved
against the now-parallel M2-T02/T03 merges, verified with a full
`cargo test --workspace` pass before pushing), and the M2-T04 builder
(which had been working against a stale `.gitkeep`-only `crates/dom`) was
notified directly to re-sync. M2-T05 is now independently re-verified
present and passing on `main`. Recorded here rather than silently corrected,
per this repo's evidence-over-narration culture.

M2-T10 still queued — next to start the moment a slot frees.

M2-T06 held back: no V8 build toolchain (`gn`/`gclient`/depot_tools) is
available locally; owner directed GitHub Actions ONLY for the V8 build
(`.github/workflows/v8-toolchain-build.yml`, workflow_dispatch). First real
run hit two fixable CI-environment bugs (depot_tools bootstrap ordering,
Windows runner disk headroom); a fix is in progress/validating. M2-T06
proper (the C++ bridge/Rust facade code) remains blocked on that workflow
producing real, checksummed artifacts.

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
