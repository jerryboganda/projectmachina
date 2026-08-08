---
title: "OpenAI Codex Entry Instructions"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Adapt the canonical autonomous workflow to Codex local, cloud, and non-interactive execution."
---

# OpenAI Codex Entry Instructions

Read `AGENTS.md`; it is authoritative. This file maps Project Machina to Codex.

## Recommended operating mode

- Use Codex locally for repository-aware implementation and review.
- Use a separate worktree for every concurrent Codex task.
- Use non-interactive execution only for task packets whose commands, permissions, and acceptance checks are explicit.
- Retain `AGENTS.md` at repository root and optionally add nested `AGENTS.md` files only when a subsystem needs stricter local rules.
- Package repeated workflows as skills under `.agents/skills/`.

## Startup prompt

```text
Act as the Project Machina orchestrator. Follow root AGENTS.md and the selected task packet.
Verify dependencies and ownership, create an isolated worktree, implement only the claimed
scope, run the fast gate, independently inspect the diff, create a reviewable commit, update
durable state, and continue the queue. Never silently substitute Chromium behavior for a
native capability claim.
```

## Parallel Codex sessions

Two Codex sessions may run at once when:

- they have distinct task IDs and worktrees;
- their write scopes do not overlap;
- neither changes an unmerged shared contract needed by the other;
- each writes a task heartbeat;
- integration order is recorded in `agents/WORK_QUEUE.md`.

## Automation safety

Default to repository-scoped filesystem access and least-privilege network access. Commands that mutate production, secrets, cloud billing, protected branches, or release tags require the human gate in `agents/HUMAN_APPROVALS.md`.

## Completion behavior

A Codex task is not complete at code generation. It must produce an evidence record, pass the task fast gate, receive independent review, merge, and update state. When context or execution ends, write the handoff before stopping.
