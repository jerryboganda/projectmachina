---
title: "Risk-Tiered Fast Gates and Final Heavy Campaign"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture Review"
purpose: "Record an accepted architecture decision for Project Machina."
---

# ADR-006 — Risk-Tiered Fast Gates and Final Heavy Campaign

## Status
Accepted.

## Context
The owner wants to minimize repeated long testing. Running exhaustive suites on every change is slow, while postponing all validation creates compounding integration failures.

## Decision
Require narrow per-task formatting, compile/type, changed tests, contracts, focused smoke, and immediate security-sensitive checks. Run selected scheduled suites. Consolidate exhaustive WPT, differential, conformance, fuzz, load, soak, chaos, security, accessibility, DR, and release rehearsal into M9.

## Consequences

- Fast inner loop with early detection of structural defects.
- Deferred-risk inventory must remain accurate.
- Final campaign may still require repair cycles; release is not automatic merely because features are complete.
