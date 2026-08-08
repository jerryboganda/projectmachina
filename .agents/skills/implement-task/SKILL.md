---
name: implement-task
description: Implement one ready Project Machina task in an isolated worktree and return reviewed, resumable evidence.
---

# Implement a Project Machina task

1. Read root `AGENTS.md`, owner decisions, current state, queue, and the exact task packet.
2. Verify dependencies and atomic path ownership. Do not proceed on an overlapping live claim.
3. Create or reuse the task branch/worktree from the recorded base commit.
4. Load only directly relevant architecture, protocol, security, and quality documents.
5. Write a short plan mapped to each acceptance criterion.
6. Implement the smallest coherent change inside the allowed paths.
7. Run the task fast gate from `quality/FAST_INNER_LOOP.md`.
8. Self-review the complete diff for correctness, security, compatibility, migration, observability, and scope.
9. Commit with the task ID and produce the completion evidence defined in `agents/TASK_PACKET_TEMPLATE.md`.
10. Request independent review; do not self-approve critical work.
11. On interruption, write the standardized handoff before the run ends.

Never run broad final suites unless the task is in M8/M9. Never make an unsupported capability appear successful.
