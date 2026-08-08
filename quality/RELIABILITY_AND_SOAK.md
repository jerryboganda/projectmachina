---
title: "Reliability, Soak, and Chaos Testing"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Verify long-running stability, fault containment, recovery, fairness, and data integrity."
---

# Reliability, Soak, and Chaos Testing

## Reliability scenarios

- repeated session create/navigate/close;
- long-lived pages, workers, WebSockets, workflows;
- high churn with mixed native/Chromium/migration;
- memory pressure and fragmentation;
- control-plane/worker/network/storage restarts;
- lease expiration and stale worker;
- client disconnect/reconnect;
- cancellation at every lifecycle phase;
- rolling deploy and worker drain;
- tenant quota/noisy-neighbor contention.

## Soak tiers

- Development smoke: 15–30 minutes for affected lifecycle changes.
- M8 qualification: 4–8 hours mixed load.
- M9 broad: 24 hours.
- M9 selected production-like: 72 hours.

## Observations

Memory/FD/thread/task/timer growth, V8 heap, DOM objects, connection pools, queue latency, CPU drift, crash, retries, worker recycle, event gaps, session leaks, artifact backlog, database/Redis/object-store health, and tenant fairness.

## Chaos injections

Kill native/Chromium worker, terminate renderer, drop network, fail DNS/proxy, slow object/database, expire grant, partition scheduler/worker, fill ephemeral disk, deny artifact write, roll configuration/version, and simulate region pool loss.

## Invariants

- No cross-tenant data.
- Durable lifecycle resolves to a terminal/recoverable state.
- Idempotent requests do not duplicate resources.
- Side-effecting actions are not repeated unsafely.
- Queue/backpressure remains bounded.
- Recovery does not weaken policy.
- Metrics count failures accurately.

## Exit

No unresolved critical/high leak, corruption, unsafe replay, deadlock, unbounded growth, or recovery failure. Tail latency/resource regressions are explained and within release budgets.
