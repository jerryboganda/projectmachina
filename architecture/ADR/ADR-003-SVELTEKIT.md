---
title: "Svelte 5 and SvelteKit"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture Review"
purpose: "Record an accepted architecture decision for Project Machina."
---

# ADR-003 — Svelte 5 and SvelteKit

## Status
Accepted.

## Context
The platform needs a fast console, real-time trace UI, accessible workflows, and static documentation without an unnecessarily heavy client.

## Decision
Use Svelte 5, SvelteKit, and TypeScript. Prerender static routes and use a server adapter for authenticated routes. Generate API clients/types.

## Consequences

- Compact compiled UI and one framework for docs/console.
- Team must enforce accessibility, bundle, and event-buffer budgets.
- Embedded widgets may use plain Svelte only by explicit need.
