---
title: "Branching, Pull Requests, and Merge Queue"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define task branches, worktrees, ownership, reviews, commits, and dependency-aware integration."
---

# Branching, Pull Requests, and Merge Queue

## Branches

- `main`: always releasable at current maturity; protected.
- `agent/<task-id>-<slug>`: one active task/worktree.
- `release/<version>`: only when stabilization requires it.
- `security/<private-id>`: restricted process for sensitive fixes.

Avoid long-lived subsystem branches. Feature flags and small vertical tasks reduce merge delay.

## Commit style

Each commit is coherent and reviewable. Include task ID in subject/body. Do not mix formatting of unrelated files, generated artifacts without source, or speculative refactors.

Example:

```text
M2-T04: implement streaming HTML tokenizer skeleton

- add bounded input buffer and tokenizer states
- expose typed parse errors and metrics
- add focused fixtures and fuzz seed target
```

## Pull request template

- Task/requirement/capability IDs.
- Objective and scope.
- Owned paths.
- Architecture/security impact.
- Changed behavior and limitations.
- Fast-gate commands/results.
- Deferred heavy tests.
- Screenshots only for frontend where useful and non-sensitive.
- Rollback/feature flag.
- Handoff/next task.

## Required review

One independent reviewer minimum. Add security for auth/network/sandbox/secrets/unsafe/FFI, protocol for public contract, performance for benchmark-critical changes, and frontend accessibility owner for design-system/critical interaction changes.

## Merge method

Use merge queue with squash or rebase policy chosen in M0; preserve task traceability. Never merge stale base without final fast gate. Dependency tasks merge before consumers.

## Labels

`milestone:M0..M9`, `agent:ready`, `agent:claimed`, `agent:blocked`, `risk:security`, `risk:protocol`, `risk:performance`, `contract-change`, `human-gate`, `deferred-heavy-test`, `release-blocker`.

## Prohibited operations

No force push to protected branch, no deletion of another agent branch/worktree, no blanket conflict resolution, no bypassing failed required check, and no merge with unrecorded overlapping ownership.
