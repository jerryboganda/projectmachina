---
title: "Cloud Operations and Fleet Management"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define managed-service regions, worker pools, autoscaling, maintenance, tenancy, cost, and emergency controls."
---

# Cloud Operations and Fleet Management

## Regional model

Launch with one approved region while identifiers/data models remain region aware. Keep page execution and sensitive artifacts in the selected region unless policy explicitly permits movement. Control-plane replication and failover commitments are documented before sale.

## Worker pools

Separate by:

- native vs Chromium;
- shared, dedicated, hardened isolation;
- CPU/memory resource class;
- architecture and runtime version;
- proxy/network class;
- customer-dedicated pools where offered.

## Capacity

Maintain warm targets based on arrival rate, startup time, tail queue, and budget. Scale using class-specific queue and utilization. Reserve capacity for system recovery/synthetic health without starving tenants.

## Maintenance

Canary new engine/browser/kernel images, drain pools, enforce maximum worker age, patch security dependencies rapidly, rotate credentials/certificates, and verify emergency blocks. Avoid in-place mutation of workers.

## Observability

Regional and global views of verified success, queues, session lifecycle, crashes, fallback, capability misses, CPU/memory, network, artifacts, database, Redis, object storage, and cost. Alerts tie to SLOs or security signals, not raw noise alone.

## Tenant operations

Quota changes, suspension, key rotation, data export/deletion, artifact access, dedicated pool assignment, and policy overrides are authorized and audited.

## Emergency controls

- stop accepting new sessions globally/region/pool;
- force engine policy or disable capability;
- block destination/tenant/credential;
- drain/terminate worker version;
- disable artifact capture/download;
- roll back deployment/config;
- enter read-only/control-plane protection mode.

## Cost controls

Per-tenant metering, idle warm-pool budget, autoscale caps, artifact retention, network egress monitoring, Chromium fallback cost, and anomalous usage alarms. Cost optimization cannot reduce isolation below policy.
