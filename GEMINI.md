---
title: "Gemini and Google AI Studio Entry Instructions"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Adapt canonical project instructions to Gemini-backed coding agents and AI Studio managed-agent environments."
---

# Gemini and Google AI Studio Entry Instructions

Read `AGENTS.md`; it is authoritative. This file applies to Gemini CLI-style agents and managed coding agents configured through Google AI Studio.

## Environment configuration

Provision a reproducible environment with:

- Git, Rust toolchain, CMake/Clang, Node.js, pnpm, Docker, and required protocol generators;
- repository-scoped credentials only;
- explicit network allowlists;
- time and cost limits;
- durable artifact storage for logs, traces, patches, and handoffs;
- no production credentials during ordinary implementation.

## Agent prompt

```text
Read AGENTS.md and reconcile agents/CURRENT_STATE.md with repository reality. Select and
claim one ready task. Work in an isolated branch/worktree, respect file ownership, execute
the fast gate, return structured evidence, obtain independent review, merge safely, update
state, and repeat. Use recommended owner defaults unless a human-required decision is hit.
```

## Managed-agent supervision

Configure checkpoints at:

- first plan for a high-risk task;
- request for new external credentials;
- proposed dependency with unresolved license;
- change to sandbox, authentication, secret handling, or network egress;
- production deployment or release authorization.

Routine compilation, local tests, documentation, and reversible implementation should proceed without human interruption.

## Handoff

Managed runs may be terminated by time, quota, or environment lifecycle. Before termination, update `agents/CURRENT_STATE.md` and emit the standardized handoff. Another supported agent must be able to resume without access to the prior chat transcript.
