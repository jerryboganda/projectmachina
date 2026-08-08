---
title: "AI Coding Tool Compatibility and Parallel Use"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Agentic Development and Program"
purpose: "Define how Antigravity, Google AI Studio, Claude Code, Codex, and GitHub Copilot consume one repository contract and collaborate safely."
---

# AI Coding Tool Compatibility and Parallel Use

## Goal

Any supported coding agent must be able to start, stop, and resume Project Machina without relying on private chat history. Two different tools may work simultaneously when they use separate branches/worktrees, claim non-overlapping write scopes, and coordinate through the repository state machine.

The repository, not a vendor-specific conversation, is the control plane.

## Canonical instruction hierarchy

| Priority | Source | Purpose |
| --- | --- | --- |
| 1 | `AGENTS.md` | Tool-neutral non-negotiable rules |
| 2 | `OWNER_DECISIONS.md` | Human-owned choices and recommended defaults |
| 3 | `agents/CURRENT_STATE.md` and `agents/WORK_QUEUE.md` | Durable execution state |
| 4 | Milestone/task packet | Exact scope, dependencies, evidence, and fast gate |
| 5 | ADRs and subsystem documents | Architecture and interface constraints |
| 6 | Tool entry file | Tool-specific mechanics only |
| 7 | Specialist profile/skill/prompt | Bounded role behavior |

A tool-specific instruction may tighten safety or execution mechanics. It may not contradict the canonical architecture or acceptance criteria.

## Supported tool mapping

| Tool | Primary entry | Native repository features used | Recommended role |
| --- | --- | --- | --- |
| Google Antigravity | `ANTIGRAVITY.md`, `antigravity/agents.md`, `antigravity/skills.md` | Agent personas, skills, workflow/pipeline, mission-control coordination | Orchestrator plus specialist lanes |
| Google AI Studio managed agents | `AI_STUDIO.md`, `GEMINI.md`, `.agents/skills/` | Mounted `AGENTS.md`, skills, isolated Linux environment, configured tools | Bounded remote implementation/review runs |
| Claude Code | `CLAUDE.md`, `.claude/agents/` | Persistent project instructions, subagents, dynamic workflows, optional agent teams/hooks | Lead, specialists, independent reviewers |
| OpenAI Codex | `CODEX.md`, `AGENTS.md`, `.agents/skills/` | Repository instructions, skills, local/cloud execution, worktrees, non-interactive tasks | Implementation, review, migration, benchmark automation |
| GitHub Copilot | `.github/copilot-instructions.md`, `.github/agents/`, `.github/prompts/` | Repository instructions, custom agents, prompt files, GitHub issue/PR workflow, selected MCP tools | Issue-to-PR tasks, code review, CI repair, documentation |
| Human engineer | `START_HERE.md` and same task graph | Git, code review, local tools | Owner decisions, sensitive approvals, escalation |

## Two-tool operating patterns

### Pattern A — two independent implementation lanes

Use for tasks with no shared contract dependency.

```text
Tool A: task Mx-Ta, worktree ../machina-Mx-Ta, owned paths A
Tool B: task Mx-Tb, worktree ../machina-Mx-Tb, owned paths B
Merge queue: contract/low-level dependency first, then dependent task
```

Good pairings include:

- Rust native engine + Svelte console;
- protocol adapter + security documentation/tests;
- SDK generator + platform deployment;
- benchmark harness + unrelated product UI.

### Pattern B — implementer plus independent reviewer

Use when write scopes would overlap or a task is security/compatibility critical.

```text
Tool A implements and records evidence.
Tool B reads the task, contracts, diff, and evidence; it does not inherit Tool A's reasoning.
Tool A or a repair agent addresses findings.
The merge queue verifies the final diff and fast gate.
```

This is the default for unsafe Rust, V8 FFI, sandboxing, authentication, secret handling, egress, public protocol semantics, release tooling, and performance claims.

### Pattern C — contract owner plus downstream implementers

One agent owns the schema/contract change. Other agents may prepare read-only plans, tests, or adapters but do not merge against an unstable generated contract. Once the contract branch merges, downstream tasks rebase and proceed.

## Atomic claim protocol

Every agent must update the task record with:

```yaml
task_id: M3-T07
state: claimed
claimed_by: codex/session-or-run-id
branch: agent/M3-T07-short-slug
worktree: ../machina-M3-T07
base_commit: <sha>
write_paths:
  - crates/semantic/**
  - tests/semantic/**
read_paths:
  - crates/dom/**
heartbeat_at: 2026-08-08T12:00:00+05:00
lease_expires_at: 2026-08-08T12:30:00+05:00
```

The claim operation must fail when another live claim overlaps. A stale lease is reclaimed only after reconciling the branch, worktree, open pull request, latest commit, and handoff record.

## Cross-tool handoff envelope

Every task or session handoff contains:

- task ID, state, branch, worktree, and base/head commits;
- accepted write scope and actual changed paths;
- completed acceptance criteria and evidence links;
- commands and exact outcomes;
- failing checks with minimal reproduction;
- decisions, assumptions, and unresolved risks;
- generated or migrated contracts;
- next safe action;
- whether credentials, approvals, or external state are involved.

A receiving agent first verifies repository facts. It never blindly trusts prose from another model.

## Context strategy

- Load the task packet and direct dependencies, not the entire corpus.
- Use the manifest and links to fetch subsystem documents on demand.
- Summarize tool output into evidence files rather than carrying logs in conversation.
- At context pressure, create a handoff and start a fresh session.
- Preserve exact errors, commands, commit hashes, and artifact hashes.
- Do not let one model's speculative explanation become durable fact without code/test evidence.

## Tool-specific notes

### Antigravity and AI Studio

Official Google materials describe repository-loaded `AGENTS.md` and skill folders, specialized personas, workflows, and isolated managed-agent environments. Project Machina maps one bounded task to one run/environment and keeps orchestration state in Git so a new environment can resume it.

### Claude Code

Subagents are appropriate for isolated specialist work and context-heavy research. Agent teams can coordinate several sessions but are documented as experimental and have lifecycle limitations, so Project Machina does not rely on their internal task store as the only durable state. Dynamic workflow scripts are preferred for repeatable loops.

### Codex

The root `AGENTS.md` and repo-local `.agents/skills/` encode stable policy and repeated procedures. Non-interactive execution is allowed only when task scope, permissions, commands, timeouts, and acceptance evidence are explicit.

### GitHub Copilot

Repository custom instructions provide persistent project context, while `.github/agents/*.agent.md` profiles specialize recurring roles. Custom agents receive the minimum required tools. Copilot-created pull requests follow the same task claim, evidence, review, and merge rules as every other agent.

## Forbidden parallelism

Do not run two writers concurrently on:

- the same file or generated-output source;
- one public schema before the contract branch merges;
- the same database migration sequence;
- the same dependency lockfile unless coordinated;
- the V8 revision/snapshot manifest;
- release tags or production manifests;
- global formatting or mass rename work;
- benchmark baselines during a frozen certification run.

## Completion and continuity guarantee

The documentation guarantees a **resumable process contract**, not that a commercial model session will run forever. Quotas, context limits, tool failures, credentials, and human approvals can interrupt an individual run. The orchestrator achieves continuous development by persisting state, bounding each task, automatically retrying transient failures, handing off exhausted work, and continuing independent ready tasks.

## Official references

- Google Antigravity autonomous pipelines codelab: https://codelabs.developers.google.com/autonomous-ai-developer-pipelines-antigravity
- Google AI Studio managed agents: https://ai.google.dev/gemini-api/docs/aistudio-agents
- Claude Code subagents: https://code.claude.com/docs/en/sub-agents
- Claude Code parallel agents: https://code.claude.com/docs/en/agents
- Claude Code agent teams: https://code.claude.com/docs/en/agent-teams
- OpenAI Codex: https://developers.openai.com/learn/codex
- OpenAI repository skills pattern: https://developers.openai.com/blog/skills-agents-sdk
- GitHub Copilot customization: https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/customize-copilot-overview
- GitHub custom agents: https://docs.github.com/en/copilot/reference/custom-agents-configuration
