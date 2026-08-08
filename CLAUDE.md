---
title: "Claude Code Entry Instructions"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Adapt the canonical autonomous workflow to Claude Code, subagents, hooks, and agent teams."
---

# Claude Code Entry Instructions

Read `AGENTS.md`; it is authoritative. This file maps the Project Machina workflow to Claude Code.

## Recommended operating mode

- Use the primary session as orchestrator and integrator.
- Use project subagents from `.claude/agents/` for bounded research, review, security, performance, protocol, frontend, and release roles.
- When agent teams are available and enabled, assign each implementing teammate a distinct worktree and non-overlapping task claim.
- Use hooks only for deterministic policy enforcement and state capture; do not rely on conversational reminders for required checks.

## Session startup

```text
Read AGENTS.md, agents/CURRENT_STATE.md, agents/WORK_QUEUE.md, and the selected task.
Reconcile state against Git. Claim exactly one ready task. Use a separate worktree. Execute
the autonomous loop through reviewed merge, update durable state, and continue.
```

## Subagent delegation

Delegate outcomes, not vague topics. Each request must include:

- task ID;
- owned paths;
- read-only paths;
- acceptance criteria;
- prohibited changes;
- expected evidence;
- return format.

A subagent that cannot commit in an isolated worktree returns a patch plan and findings only. The parent validates all output before merge.

## Hook policy

Recommended repository hooks:

- before file edit: reject edits outside claimed scope;
- before command: reject destructive Git and unrestricted production commands;
- after edit: record changed paths in task state;
- before task completion: require fast-gate evidence and clean status;
- on stop/session end: write a resumable handoff.

Hook failure is a policy failure, not permission to bypass the hook.

## Context management

Use `/compact` or a fresh session before context quality degrades. First write a handoff conforming to `agents/HANDOFF_PROTOCOL.md`. The next session must trust repository evidence over a prose summary.

## Review separation

The implementation subagent must not serve as the only reviewer. Assign the reviewer profile in `.claude/agents/reviewer.md`, or use a second tool/owner. Security-sensitive work also invokes `.claude/agents/security.md`.
