---
title: "Fleet Scheduler and Isolation"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define worker placement, tenant fairness, leases, capacity, and three security/performance isolation tiers."
---

# Fleet Scheduler and Isolation

## Scheduling inputs

- engine and capability requirements;
- isolation tier;
- region/data policy;
- architecture/runtime version;
- proxy/network policy;
- expected resource class;
- tenant quota and fairness weight;
- warm-pool availability;
- session affinity/persistent profile needs;
- current fleet health.

## Isolation tiers

### Shared-performance

Multiple trusted/low-risk sessions share a worker process with separate V8 contexts/engine state. Fastest startup and density. Strict logical isolation, quotas, and policy still apply. Not the default for untrusted multi-tenant public workloads unless security review approves.

### Dedicated-process

One tenant session or small same-tenant session group per worker process. OS user/namespaces/cgroups and scoped filesystem/network policy. Recommended managed-service default.

### Hardened

Dedicated container or microVM, minimized image, strong kernel boundary, dedicated network namespace, read-only root, ephemeral storage, and stricter egress. Used for enterprise/untrusted/high-risk workloads.

## Placement algorithm

1. Filter workers by health, region, engine version, architecture, isolation, and policy.
2. Exclude workers near hard resource/recycle thresholds.
3. Apply tenant quota and queue fairness.
4. Prefer warm compatible capacity while bounding idle cost.
5. Score locality, fragmentation, expected resource class, and profile affinity.
6. Grant a session lease; worker acknowledges before session becomes ready.

## Backpressure

Queues are bounded per tenant and globally. Return queue position/estimate class, not false readiness. Reject with typed quota/capacity errors when policy requires. High-priority internal repair traffic cannot starve customer work indefinitely.

## Leases and heartbeats

Control plane grants worker/session leases. Missing heartbeats trigger suspect then lost state. Reassignment occurs only when idempotency and external side effects permit. Durable session metadata records the terminal classification.

## Worker recycling

Recycle after age, session count, memory fragmentation, crash count, V8/Chromium health signal, or version change. Draining workers accept no new sessions and finish within a deadline before forced close.

## Autoscaling

Scale on queued demand by engine/isolation/resource class, warm-pool target, startup time, and cost ceiling. Use separate pools for native and Chromium. Avoid one aggregate CPU signal that masks constrained classes.

## Noisy-neighbor controls

Per-session/task CPU budgets, memory limits, request/byte limits, bounded queues, fair scheduling, and worker eviction. Attribute usage to tenant/project/session for both enforcement and billing.
