---
title: "Multi-Agent Git Worktrees and Claims"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture Review"
purpose: "Record an accepted architecture decision for Project Machina."
---

# ADR-005 — Multi-Agent Git Worktrees and Claims

## Status
Accepted.

## Context
Two coding agents can accelerate independent work but shared checkouts and overlapping edits create corruption and hidden conflicts.

## Decision
Use one branch/worktree per active task, atomic task/path claims, leases/heartbeats, contract-first sequencing, and one merge queue. Default to two implementation lanes.

## Consequences

- Safe cross-tool concurrency and resumability.
- Requires M0 ownership tooling and disciplined task boundaries.
- Shared schemas/build files are serialized.
