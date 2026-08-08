---
title: "Agent Orchestration"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define the project-level controller that keeps multiple coding agents aligned and continuously productive."
---

# Agent Orchestration

## Objective

The orchestrator advances the dependency graph, not merely individual conversations. It owns task selection, claims, environment preparation, evidence validation, merge ordering, state reconciliation, escalation, and project-complete evaluation.

## Authority boundary

The orchestrator may:

- choose any ready task using the priority policy;
- assign one or two implementation lanes;
- create branches and worktrees;
- launch specialist and reviewer agents;
- retry failed bounded steps;
- merge changes that satisfy policy;
- update task state and select subsequent work.

It may not:

- accept legal, licensing, privacy, or material security risk;
- authorize production access or spend;
- approve a public performance claim;
- bypass protected-branch, review, or required security controls;
- mark an acceptance criterion complete without evidence.

## Truth reconciliation

At the start of every orchestration cycle, compare:

1. Git default branch and commit graph.
2. Existing worktrees and branches.
3. Open pull requests and review state.
4. CI/check status.
5. Task claims and heartbeats.
6. `agents/CURRENT_STATE.md` and `agents/WORK_QUEUE.md`.
7. Capability and acceptance evidence.

Repository and service facts take precedence over stale prose. Correct the prose and record the reconciliation event.

## Ready-task selection

A task is ready when:

- all hard dependencies are complete;
- no human gate is pending for that task;
- its write scope can be reserved without overlap;
- required tools and credentials are available;
- its milestone has begun or the task is explicitly marked cross-milestone;
- no unresolved architecture conflict makes implementation unsafe.

Order ready tasks by:

1. critical security or build repair;
2. task that unblocks the largest number of descendants;
3. shared contract before dependent implementations;
4. highest milestone priority;
5. shortest estimated critical-path duration;
6. oldest ready timestamp.

## Two-lane scheduling

The default lanes are:

- **Lane A — critical path:** shared contracts, engine foundations, release blockers.
- **Lane B — parallel value:** independent protocol adapters, console, tests, operations, or research-backed implementation.

Do not put two tasks in parallel when both touch a shared schema, generated output source, root build configuration, or the same subsystem ownership boundary.

## Agent invocation packet

Every invocation includes:

```yaml
task_id: Mx-Tyy
base_commit: <sha>
branch: agent/<task-id>-<slug>
worktree: <absolute-or-resolved-path>
role: <role-id>
write_scope:
  - path/**
read_scope:
  - '**'
forbidden_scope:
  - <paths>
dependencies:
  - <completed-task-id>
acceptance_source: planning/<milestone>.md#<task>
fast_gate_source: quality/FAST_INNER_LOOP.md
repair_limit: 3
human_gates:
  - <gate-id-or-none>
```

## Merge queue

Merge in dependency order. Before merge, verify:

- branch is rebased or updated against current main;
- ownership remained valid;
- fast gate passed on the final diff;
- review findings are resolved or documented with accepted waiver;
- generated files are current;
- public contracts and docs changed together;
- no secret or sensitive trace is present;
- state update is part of the merge or immediately follows atomically.

## Project-complete evaluation

The orchestrator may declare `COMPLETE_CANDIDATE` only after every required task is complete. It may declare `COMPLETE` only after M9 final certification, owner release approvals, reproducible artifact generation, rollback rehearsal, and closure of all critical/high findings.
