---
title: "Google Antigravity Entry Instructions"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Adapt the canonical workflow to Antigravity workspaces, agents, skills, mission control, and autonomous pipelines."
---

# Google Antigravity Entry Instructions

Read `AGENTS.md`; it is authoritative. Configure Antigravity as a mission-control layer over bounded, repository-backed tasks.

## Workspace model

- One project workspace tracks the main repository.
- One agent execution maps to one task claim and worktree.
- Reusable operations map to skills described in `antigravity/skills.md`.
- The mission-control board mirrors `agents/WORK_QUEUE.md`; repository state remains authoritative when the two disagree.
- Agent artifacts are stored under the task evidence prefix and linked from the task record.

## Pipeline

```text
reconcile repository state
  -> select ready task
  -> reserve task and paths
  -> prepare worktree/environment
  -> invoke specialist agent
  -> run fast gate
  -> invoke independent reviewer
  -> bounded repair
  -> merge queue
  -> persist evidence/state
  -> select next task
```

## Autonomy policy

Antigravity may automatically repeat the pipeline for ready tasks. It must pause only when `agents/HUMAN_APPROVALS.md` requires an accountable owner, when external credentials are missing, or when bounded repairs are exhausted. A failed task does not stop unrelated ready tasks.

## Recommended first use

Run M0 with one orchestrator and one builder. Enable a second implementation lane after the ownership and worktree automation from M0 is proven. Use the workflow in `antigravity/autonomous-build-workflow.md` as the pipeline definition.
