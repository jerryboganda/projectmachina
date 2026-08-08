---
title: "Capacity and Cost Management"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Plan worker capacity, warm pools, quotas, metering, fallback cost, and economic efficiency."
---

# Capacity and Cost Management

## Capacity units

Track by engine, isolation tier, resource class, region, version, and proxy/network class. One aggregate “browser slot” is too imprecise.

## Demand measures

Arrival rate, concurrent sessions, task duration, queue wait, native/fallback mix, cold starts, CPU, memory, network, persistent profile use, workflow schedules, and tenant burst patterns.

## Warm pools

Set minimum/maximum warm workers from startup latency target and idle-cost ceiling. Native and Chromium pools have separate economics. Keep hardened capacity small/on-demand unless SLO requires warm instances.

## Quotas

Per organization/project/API key:

- requests and session starts;
- concurrent queued/active sessions;
- CPU/memory time;
- network requests/bytes;
- Chromium/fallback minutes;
- artifact/storage bytes and retention;
- workflow runs and recovery/LLM budget.

## Metering

Emit immutable usage events with tenant/project/session, engine/isolation, resource quantities, version, and correction/idempotency identity. Aggregate for dashboards/billing while retaining reconciliation ability.

## Unit economics

Report cost per 1,000 verified tasks, not merely per session. Include idle warm capacity, retries, failed tasks, fallback, storage, network, control-plane, and support overhead.

## Optimization priorities

1. Improve verified success and reduce retries.
2. Route correctly before launching wrong engine.
3. Reduce unnecessary fidelity/resources.
4. Improve native coverage for high-cost fallback capabilities.
5. Tune snapshots/warm pools/density.
6. Reduce artifact/telemetry waste.
7. Negotiate/infrastructure optimization after software behavior is understood.

## Budget safeguards

Autoscale caps, tenant spend limits, fallback budget, artifact retention, anomalous egress alerts, and human approval for material production spend change.
