---
title: "Start Here — Autonomous Build Bootstrap"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Provide a deterministic startup sequence for a human owner or coding agent."
---

# Start Here

This file is the canonical entry point for a new repository, new agent session, or tool handoff.

## Owner bootstrap

1. Create a private Git repository using the directory structure in `architecture/REPOSITORY_STRUCTURE.md`.
2. Copy this documentation pack into `/docs/` or retain it at repository root. If placed under `/docs`, update path references once in the bootstrap pull request.
3. Review `OWNER_DECISIONS.md`. Unanswered decisions use the marked recommended defaults.
4. Protect `main`: require a pull request, one independent review, fast-gate success, and no unresolved critical security finding.
5. Create labels listed in `delivery/BRANCHING_AND_MERGE.md`.
6. Configure two Git worktrees for concurrent implementation, following `agents/MULTI_AGENT_CONCURRENCY.md`.
7. Start the selected coding agent with the **Bootstrap prompt** below.


## Tool-native launch files

| Tool | Start with | Specialist assets |
| --- | --- | --- |
| Antigravity | `ANTIGRAVITY.md` | `antigravity/agents.md`, `antigravity/skills.md`, `antigravity/autonomous-build-workflow.md` |
| Google AI Studio / Gemini | `AI_STUDIO.md` and `GEMINI.md` | `.agents/skills/` |
| Claude Code | `CLAUDE.md` | `.claude/agents/` |
| OpenAI Codex | `CODEX.md` and `AGENTS.md` | `.agents/skills/` |
| GitHub Copilot | `.github/copilot-instructions.md` | `.github/agents/`, `.github/prompts/` |

Two tools may run concurrently only through the worktree, task-claim, path-ownership, and handoff protocol. The recommended default is two implementation lanes plus an independent reviewer.

## Bootstrap prompt

Copy this prompt into Antigravity, Claude Code, Codex, Copilot, or an AI Studio coding agent:

```text
You are the Project Machina orchestrator. Read AGENTS.md first, then START_HERE.md,
agents/ORCHESTRATION.md, agents/AUTONOMOUS_LOOP.md,
planning/MASTER_TASK_GRAPH.md, planning/DEPENDENCY_MAP.md, and OWNER_DECISIONS.md.

Use accepted defaults for unanswered owner decisions. Do not ask broad planning questions.
Inspect the repository, reconcile agents/CURRENT_STATE.md with Git and CI, select the
highest-priority ready task, create or use an isolated worktree, and execute the complete
autonomous loop. Continue selecting ready tasks until an explicit human-approval gate,
a hard external dependency, or the project-complete condition is reached.

Use only non-overlapping file ownership for concurrent agents. Record every claim,
decision, command, test result, pull request, and handoff in repository state. Apply the
smallest fast gate during development and defer the exhaustive campaign to M9, without
skipping compilation, contract, security-sensitive, or changed-code checks.
```

## New-session resume prompt

```text
Resume Project Machina from durable repository state. Read AGENTS.md and
agents/CURRENT_STATE.md. Verify the state against Git branches, worktrees, open pull
requests, task claims, and CI results. Do not repeat completed work. Reclaim an abandoned
task only according to agents/FAILURE_RECOVERY.md. Continue the highest-priority ready
task through review, merge, and state update, then loop.
```

## Two-agent startup

Use one of these role pairings:

| Agent A | Agent B | Best stage |
| --- | --- | --- |
| Architect/orchestrator | Platform bootstrap engineer | M0–M1 |
| Native-engine engineer | Protocol/control-plane engineer | M2–M4 |
| Agent-runtime engineer | Svelte frontend engineer | M5–M6 |
| Security/platform engineer | Performance/quality engineer | M7–M8 |
| Release lead | Independent certification reviewer | M9 |

Both agents must use separate branches and worktrees. They may read the entire repository but may edit only their claimed file scopes. Shared contracts require a designated contract owner and are merged before dependent implementations.

## First task

Unless repository inspection shows it already complete, begin with `M0-T01` in `planning/MILESTONE_00_FOUNDATION_AND_GOVERNANCE.md`. Its output establishes the monorepo, toolchain pins, baseline CI, agent-state directory, and ownership tooling needed by all later work.

## Do not begin implementation when

Stop only when one of these conditions is true:

- the task requires an owner decision marked `human-required`;
- required credentials or infrastructure are unavailable;
- legal review is required for a dependency or license;
- the same acceptance failure persists after three bounded repair attempts;
- continuing would weaken a security control or corrupt durable state;
- no task is ready because an external dependency is unresolved.

Record the exact blocker, evidence, impact, and recommended choice. Continue all unaffected work.
