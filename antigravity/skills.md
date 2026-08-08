---
title: "Antigravity Skills Catalog"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Agentic Development"
purpose: "Define reusable Antigravity skills that implement the Project Machina autonomous development state machine."
---

# Antigravity Skills Catalog

## Skill loading rule

Skills implement repeatable mechanics; they do not redefine project architecture. Each skill reads root `AGENTS.md`, the active task packet, and the minimum linked context. The same procedures are mirrored under `.agents/skills/` for cross-tool use.

| Skill | Trigger | Input | Output |
| --- | --- | --- | --- |
| `reconcile-project-state` | Start/resume, stale lease, interrupted run | Git/worktrees/PR/CI/task files | Reconciled queue and safe recovery actions |
| `claim-task` | Ready task selected | Task ID, agent/run, desired paths | Atomic claim or overlap rejection |
| `prepare-worktree` | Claim succeeds | Base commit, task ID | Isolated branch/worktree and environment manifest |
| `implement-task` | Bounded task ready | Task packet and claimed scope | Commit/patch and implementation evidence |
| `fast-gate` | Implementation changes exist | Changed paths and task commands | Structured pass/fail report |
| `independent-review` | Fast gate passes | Task, contracts, diff, evidence | Blocking/non-blocking findings |
| `bounded-repair` | Review/CI finding exists | Finding and reproduction | Narrow repair or exhausted status |
| `merge-and-checkpoint` | Review passes | PR/commit and evidence | Merged state, released claim, next task |
| `handoff` | Context/session/lease ending | Current run state | Durable cross-tool handoff |
| `benchmark` | Performance task | Frozen build and benchmark manifest | Raw data, statistics, comparison report |
| `final-certification` | M9 candidate frozen | Candidate digest and release plan | Complete evidence index or release blockers |

## Rework loop

```text
implementation
  -> fast gate
  -> independent review
  -> repair 1
  -> review
  -> repair 2
  -> review
  -> repair 3
  -> review
  -> pass OR blocked-and-escalated
```

Transient infrastructure retries are separate from repair cycles and must use bounded exponential backoff. A repair may not hide a failure, weaken a contract, skip a required check, or expand timeout limits without evidence.

## Artifact handoff

All skills return machine-readable status and human-readable Markdown evidence. Large logs, profiles, traces, test outputs, and recordings are content-addressed artifacts; the Markdown record stores hashes and locations rather than embedding them.

## Human pauses

A skill pauses only for gates in `agents/HUMAN_APPROVALS.md`: legal/license, new production credential, material security posture, destructive production operation, significant spend, public claim, release/GA, or other explicitly owner-reserved decision. Unaffected ready tasks continue.
