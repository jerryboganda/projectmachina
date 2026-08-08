---
title: "Deployment Architecture and Procedures"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define local, self-hosted, and managed deployment topologies, configuration, rollout, and rollback."
---

# Deployment

## Modes

### Local developer

Single host with local control API, one native worker, optional Chromium worker, PostgreSQL/Redis/object-store containers, and SvelteKit console. Safe defaults restrict external access.

### Self-hosted

Docker Compose for small installations and Helm/Kubernetes for production. Customers supply identity, secrets, storage, ingress, network policy, and observability integrations according to documented requirements.

### Managed cloud

Regional control/data-plane deployment with dedicated worker pools by engine/isolation class, managed data services, object storage, workload identity, egress controls, autoscaling, and central operations.

## Kubernetes units

- API/control-plane deployments.
- Scheduler/workflow/background workers.
- Native worker pools.
- Chromium worker pools.
- Hardened job/microVM integration.
- Console/docs deployments.
- Migrations as controlled jobs.

Use pod security, network policy, non-root, read-only root, resource requests/limits, disruption budgets, topology spread, probes, and dedicated node pools where required.

## Configuration

Version configuration and policy; inject secret references. Validate before rollout. Attach config version to sessions and release evidence. Emergency restrictions have audited fast propagation.

## Data migrations

Use expand/migrate/contract. Back up before destructive phases, test on production-like data, make jobs resumable/idempotent, expose progress, and retain compatible application versions until safe.

## Rollout procedure

1. Verify artifact signature/provenance.
2. Apply compatible schema expansion.
3. Deploy control-plane canary.
4. Deploy small worker canary pools.
5. Run synthetic/real authorized smoke.
6. Observe gates.
7. Gradually promote.
8. Drain/recycle old workers.
9. Complete data migration/contract only after rollback window.
10. Record release state.

## Rollback

Stop promotion, route new sessions to previous worker version, drain/terminate faulty pools, roll back compatible control plane/config, and invoke data recovery plan only when required. Do not migrate an active session across incompatible versions without support.
