---
title: "System Architecture"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define the complete hybrid browser platform, its trust boundaries, runtime paths, and deployment units."
---

# System Architecture

## Context

Project Machina exposes one browser automation product while internally using two engines:

- a machine-native engine optimized for startup, density, semantic interaction, and extraction;
- a managed Chromium compatibility engine for full rendering and unsupported behavior.

The system must make engine choice observable while keeping command semantics stable.

## Logical architecture

```text
Clients
  TypeScript | Python | Go | Java | Rust | CLI
  Playwright/Puppeteer via CDP | Selenium via BiDi | MCP clients
        |
API edge and protocol gateways
  auth | quotas | rate limits | idempotency | deadlines | audit
        |
Unified typed command bus and event model
        |
Session service ---- Policy service ---- Capability registry
        |                     |
Capability/fidelity router ---+
        |                       \
Native engine workers           Chromium workers
  V8, DOM, network,             full renderer/browser
  storage, semantic kernel      screenshots/PDF/media/GPU
        |                       /
        +---- State bridge ----+
        |
Scheduler and isolation manager
  shared workers | dedicated process | hardened container/microVM
        |
Observability, artifacts, usage metering, control plane
        |
PostgreSQL | Redis-compatible coordination | object storage
```

## Runtime planes

### Data plane

Executes sessions and page code. It includes schedulers, native workers, Chromium workers, network policy enforcement, proxies, state bridge, and event emission. It is horizontally partitionable by region and isolation class.

### Control plane

Stores organizations, projects, credentials metadata, policies, capability versions, workflows, schedules, usage aggregates, audit indexes, and deployment configuration. It never executes arbitrary page JavaScript directly.

### Developer/operator plane

SvelteKit console, CLI, SDKs, documentation, trace explorer, workflow editor, and administrative operations.

## Primary flows

### Native completion

1. Client creates session with policy.
2. Router sees native eligibility.
3. Scheduler assigns a native worker of the required isolation tier.
4. Commands execute through the command bus.
5. Semantic/extraction output and verification metadata return.
6. Usage, traces, and capability evidence are recorded.

### Pre-routed Chromium

1. Requested capability declares visual/full-browser need.
2. Router assigns Chromium without native attempt.
3. Same command model is adapted to the Chromium worker.
4. Response reports `engine=chromium`, `fallback=false`, reason `requested-capability`.

### Runtime migration

1. Native page invokes or needs unsupported behavior.
2. Native engine emits a typed capability miss.
3. Router checks policy and migration eligibility.
4. State bridge creates a Chromium context, transfers serializable state, and replays verified action history when required.
5. Post-migration checkpoints confirm URL, auth/session state, and workflow invariant.
6. Client receives migration event and final metadata.

## Trust boundaries

- Public client to API edge.
- Control plane to data-plane worker.
- Tenant to tenant.
- Session to session.
- Host to hostile page process/runtime.
- Worker to external Internet/internal network.
- Platform to secret vault.
- Platform to artifact/log stores.
- Native Rust to C++/V8 FFI.

Each boundary has authentication, authorization, validation, resource limits, and observability requirements in `security/`.

## Deployment units

| Unit | Responsibility | State |
| --- | --- | --- |
| `api-gateway` | Public HTTP/gRPC, auth, quotas, routing | Stateless |
| `session-control` | Session lifecycle, task orchestration, policy | Durable metadata |
| `scheduler` | Worker selection, fairness, capacity, leases | Ephemeral + durable checkpoints |
| `native-worker` | Native engine sessions | Ephemeral session state |
| `chromium-worker` | Chromium contexts/sessions | Ephemeral session state |
| `workflow-service` | Definitions, versions, schedules, approvals, runs | Durable |
| `artifact-service` | Redacted traces, recordings, bundles | Object storage metadata |
| `usage-service` | Metering and aggregates | Durable/event stream |
| `console` | SvelteKit application | Stateless except session/cache |
| `admin-ops` | Fleet and emergency operations | Audited |

## Architectural invariants

- External adapters never bypass the command bus.
- A capability status is versioned and evidence-backed.
- A successful response cannot represent an unsupported no-op.
- Deadlines and cancellation flow end to end.
- Page execution has a resource and isolation policy.
- Durable state is never stored only inside a worker.
- Engine migration is explicit and verifiable.
- Sensitive values are represented by references outside the moment of use.
- Public claims distinguish native, hybrid, and Chromium-only outcomes.
