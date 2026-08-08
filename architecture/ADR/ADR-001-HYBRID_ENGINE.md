---
title: "Hybrid Native Engine and Chromium Fallback"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture Review"
purpose: "Record an accepted architecture decision for Project Machina."
---

# ADR-001 — Hybrid Native Engine and Chromium Fallback

## Status
Accepted.

## Context
A fully new browser engine offers performance control but broad Web compatibility is a multi-year scope. Chromium alone provides compatibility but carries startup and resource cost for machine workloads.

## Decision
Build an independent native engine for prioritized machine workloads and a first-class Chromium compatibility engine. Route or migrate through a versioned capability router and state bridge. Expose engine/fallback metadata.

## Consequences

- Useful product can ship before native coverage is broad.
- Native work is prioritized by measured fallback value.
- State migration and dual-engine conformance add complexity.
- Marketing must distinguish native, hybrid, and Chromium-only results.
