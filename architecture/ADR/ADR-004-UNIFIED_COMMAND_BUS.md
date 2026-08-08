---
title: "Unified Typed Command Bus"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture Review"
purpose: "Record an accepted architecture decision for Project Machina."
---

# ADR-004 — Unified Typed Command Bus

## Status
Accepted.

## Context
Separate implementations for HTTP, CDP, BiDi, MCP, CLI, and agents would diverge and create silent incompatibilities.

## Decision
Define one versioned typed command/event/error model. Every adapter translates to/from it; both engines implement it.

## Consequences

- Consistent behavior and test reuse.
- Canonical model design becomes critical shared-contract work.
- Protocol-specific features require explicit extension rather than bypass.
