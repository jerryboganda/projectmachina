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

| Rank | Task | Milestone | Role | Dependencies | Suggested lane |
| ---: | --- | --- | --- | --- | --- |
| 1 | M1-T03 | M1 | platform | M1-T01, M1-T02, M0-T04 | A |

## Active

| Task | Owner | Branch/worktree | State | Heartbeat |
| --- | --- | --- | --- | --- |
None.

## In review

M0-T01 through M0-T12 and M1-T01/M1-T02 have merged hosted-gate evidence; Docker limitation
is waived for M1 under owner option B.

## Blocked

Docker/Compose health/reset remains a release limitation under the recorded waiver.

## Deferred until dependencies

See `planning/DEPENDENCY_MAP.md` and the milestone files. The orchestrator regenerates this view as tasks advance.

## Queue mutation rules

- Do not change task IDs or acceptance criteria here; change the milestone source.
- Add status, owner, branch, and evidence links only.
- Every active item needs a current claim and heartbeat.
- Every blocked item needs reproduction evidence and a next decision/action.
