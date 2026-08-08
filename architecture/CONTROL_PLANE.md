---
title: "Control Plane Architecture"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define durable services, APIs, workflows, policy, tenancy, usage, and administrative operations."
---

# Control Plane Architecture

## Responsibilities

- organizations, members, roles, projects, and credentials metadata;
- session requests and lifecycle projection;
- policies, quotas, regions, proxies, and secret references;
- workflows, versions, approvals, schedules, and run state;
- capability/build registry and routing configuration;
- worker inventory, leases, rollouts, and circuit breakers;
- audit indexes, usage metering, billing aggregates;
- artifact metadata and retention;
- administrative/emergency operations.

## Service boundaries

Start as a modular service set that can deploy as a small number of processes. Do not create network microservices for every module before scale demands it. Preserve logical boundaries and asynchronous event contracts so deployment can split later.

Recommended initial deployables:

- API gateway/control API;
- session/scheduler service;
- workflow service;
- worker agents;
- artifact/usage background workers;
- SvelteKit console.

## Data stores

### PostgreSQL

Source of truth for tenancy, policies, sessions, workflows, approvals, audit metadata, capability versions, and usage aggregates. Use transactional outbox for durable events.

### Redis-compatible store

Ephemeral locks, rate limits, queue acceleration, heartbeats, and short-lived cache. It is not the sole source of durable task/session truth.

### S3-compatible object storage

Encrypted artifacts, traces, recordings, crash/reproduction bundles, benchmark reports, and release evidence.

## Eventing

Use versioned events with event ID, aggregate ID, sequence, causation, correlation, tenant, type, schema version, timestamp, and redaction classification. Delivery is at least once; consumers are idempotent.

## Consistency

- Strong consistency for authorization, policy version selection, task claim/lease, workflow approval, and destructive state changes.
- Eventual consistency for dashboards, usage aggregates, search, and telemetry views.
- Session commands use idempotency keys and expected-version checks where relevant.

## Authorization

Central policy evaluation at API/service boundaries. Workers receive scoped signed execution grants containing session, tenant, policy hash, expiry, and allowed operations; they do not receive broad database credentials.

## Administrative operations

Every privileged action is explicit, authorized, audited, reason-coded, and preferably time-bounded: worker drain, tenant suspension, domain block, key rotation, feature disable, rollback, artifact access, retention override.
