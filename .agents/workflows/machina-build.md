---
title: Project Machina Continuous Build
description: Run the resumable autonomous task loop until an explicit gate, external blocker, or completed M9 certification.
---

# `/machina-build` Continuous Build Workflow

When invoked with optional arguments such as a milestone, task ID, or maximum concurrency, follow root `AGENTS.md`, `.agents/agents.md`, and the skills in `.agents/skills/`.

## Execution sequence

1. Execute `reconcile_project_state.md`.
2. Select up to the configured number of ready, non-overlapping tasks; default two implementation lanes.
3. For each task, execute `claim_task.md` and `prepare_worktree.md`.
4. Dispatch the matching specialist persona and execute `implement_task.md`.
5. Execute `fast_gate.md`.
6. Dispatch `@reviewer` and execute `independent_review.md`.
7. For blocking findings, execute `bounded_repair.md` and repeat independent review, maximum three cycles.
8. Execute `merge_and_checkpoint.md` in dependency order.
9. Immediately return to step 1 while ready work exists.
10. On run/context interruption, execute `handoff.md` before stopping.
11. When M0–M8 are complete, freeze the candidate and execute `final_certification.md` with `@release` and independent reviewers.
12. Stop at the human release/GA decision; never self-authorize launch.

A blocked task does not stop unrelated ready tasks. A provider session ending does not erase progress because state, commits, and evidence are durable.
