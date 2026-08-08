---
title: "Durable Data and Event Backbone"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture Review"
purpose: "Record an accepted architecture decision for Project Machina."
---

# ADR-007 — Durable Data and Event Backbone

## Status
Accepted.

## Context
The control plane needs transactions, ephemeral coordination, large classified artifacts, and resilient event propagation.

## Decision
Use PostgreSQL as durable source of truth, Redis-compatible storage for ephemeral coordination/rate limits, S3-compatible object storage for artifacts, and a transactional outbox with versioned at-least-once events.

## Consequences

- Familiar operational model and clear durability boundaries.
- Consumers must be idempotent.
- Redis is never the only copy of durable lifecycle state.
- Object retention, encryption, and access policy are first-class.
