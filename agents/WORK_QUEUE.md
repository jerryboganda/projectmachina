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
| 1 | M0-T02 | M0 | platform | M0-T01 | A |
| 2 | M0-T04 | M0 | architect/protocol | M0-T01 | B |
| 3 | M0-T03 | M0 | platform/security | M0-T01, M0-T02 | A |
| 4 | M0-T05 | M0 | architect | M0-T01, M0-T04 | B |

## Active

| Task | Owner | Branch/worktree | State | Heartbeat |
| --- | --- | --- | --- | --- |
| M0-T01 | copilot-foundation | `D:\Projects\machina-worktrees\M0-T01-bootstrap` | in-review | 2026-08-09T01:36+05:00 |

## In review

M0-T01 foundation batch is awaiting independent review before merge.

## Blocked

Docker/Compose health is pending; all available source fast-gate checks have run.

## Deferred until dependencies

See `planning/DEPENDENCY_MAP.md` and the milestone files. The orchestrator regenerates this view as tasks advance.

## Queue mutation rules

- Do not change task IDs or acceptance criteria here; change the milestone source.
- Add status, owner, branch, and evidence links only.
- Every active item needs a current claim and heartbeat.
- Every blocked item needs reproduction evidence and a next decision/action.
