---
title: "Agent Context Management"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Keep long-running agentic development accurate across context limits and tool changes."
---

# Agent Context Management

## Rule

Repository state is memory. Model context is a cache.

## Minimum context for a task

Load:

- root `AGENTS.md`;
- current state and work queue;
- the exact milestone task block;
- directly affected architecture/protocol/security contracts;
- relevant source and tests;
- open review findings.

Do not preload every document. Search by task ID, component, API symbol, and acceptance phrase.

## Context budget allocation

| Content | Suggested share |
| --- | ---: |
| Task and acceptance criteria | 15% |
| Relevant contracts and source | 35% |
| Tool output and test evidence | 20% |
| Active reasoning/implementation | 25% |
| Handoff reserve | 5% |

## Compaction checkpoint

Before compaction or session restart:

1. commit a coherent WIP state or create a patch;
2. run the smallest meaningful check available;
3. update task evidence;
4. write the standardized handoff;
5. record the exact next command/action.

## Avoiding stale context

After any merge to main that affects the task:

- fetch and inspect the changed contract;
- rebase/update the worktree;
- invalidate assumptions based on the old interface;
- rerun the relevant fast gate.

## Decision memory

Consequential decisions belong in ADRs or task decision logs. Do not rely on “the previous agent said.” Every decision should include rationale, alternatives, consequences, and reversal condition.
