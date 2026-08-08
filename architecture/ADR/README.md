---
title: "Architecture Decision Record Index"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Index accepted architectural decisions and define ADR governance."
---

# Architecture Decision Records

| ADR | Decision | Status |
| --- | --- | --- |
| ADR-001 | Hybrid native engine plus Chromium fallback | Accepted |
| ADR-002 | Rust core plus narrow C++ V8 bridge | Accepted |
| ADR-003 | Svelte 5 and SvelteKit for web surfaces | Accepted |
| ADR-004 | One unified typed command bus | Accepted |
| ADR-005 | Worktree and file-claim multi-agent development | Accepted |
| ADR-006 | Risk-tiered fast gates plus final heavy certification | Accepted |
| ADR-007 | PostgreSQL, Redis-compatible coordination, object storage, and durable events | Accepted |

## ADR rule

Create a new ADR for a hard-to-reverse decision affecting core technology, component boundaries, public protocol semantics, state durability, isolation/security, deployment topology, or major compatibility policy. Do not rewrite accepted history; supersede it with a new ADR.
