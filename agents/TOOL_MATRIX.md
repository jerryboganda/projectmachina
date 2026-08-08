---
title: "AI Coding Tool Compatibility Matrix"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Explain how supported agentic coding products participate in one portable workflow."
---

# AI Coding Tool Compatibility Matrix

The project is intentionally tool-neutral. `AGENTS.md`, task packets, Git, and durable evidence are canonical. Product-specific features accelerate the workflow but are never the only place state exists.

| Capability | Antigravity | Google AI Studio agent | Claude Code | OpenAI Codex | GitHub Copilot |
| --- | --- | --- | --- | --- | --- |
| Primary use | Mission control and autonomous pipelines | Managed bounded execution | Local orchestration, subagents, hooks, teams | Local/cloud coding, review, scripted execution | GitHub issue-to-PR and IDE/CLI assistance |
| Canonical entry | `ANTIGRAVITY.md` | `AI_STUDIO.md` + `GEMINI.md` | `CLAUDE.md` | `CODEX.md` | `.github/copilot-instructions.md` |
| Reusable role definitions | Antigravity agents/skills | Environment prompt/templates | `.claude/agents/` | `AGENTS.md` + `.agents/skills/` | `.github/agents/*.agent.md` |
| Concurrent editing | One worktree per agent | One worktree per managed run | Worktrees/agent teams | Worktrees or separate cloud tasks | One branch/PR per coding-agent task |
| Deterministic enforcement | Pipeline steps | Managed environment policy | Hooks | Wrapper scripts/CI | Branch rules, workflows, review |
| Durable handoff | Repository evidence | Artifacts + repository state | Repository handoff | Repository handoff | Issue/PR + repository handoff |

## Portability contract

A task packet must be executable without product-private memory. It contains:

- objective and task ID;
- dependencies and base commit;
- allowed paths;
- acceptance criteria;
- fast-gate commands;
- human gates;
- expected evidence and handoff format.

## Choosing a tool

- Use Antigravity when centralized mission control and repeated autonomous pipelines are the priority.
- Use AI Studio managed agents for isolated, bounded environments with explicit tools and credentials.
- Use Claude Code for repository-wide reasoning, specialized subagents, deterministic hooks, and paired local work.
- Use Codex for local/cloud implementation, scripted tasks, skills, and independent review.
- Use Copilot coding agent for well-bounded GitHub issues that can produce isolated pull requests.

## Using two tools simultaneously

Recommended default:

1. The orchestrator assigns non-overlapping ready tasks.
2. Tool A owns Lane A and Tool B owns Lane B.
3. Both use distinct worktrees and task claims.
4. Each tool reviews the other tool's pull request where domain expertise permits.
5. A single merge queue serializes integration.

Do not have two tools “race” on the same task unless conducting an explicitly budgeted comparative prototype. Racing wastes tokens and creates ambiguous ownership.
