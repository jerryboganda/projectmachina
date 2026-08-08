---
title: "Platform Operations Runbook"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Provide the first-response index for common service, worker, data, network, and release incidents."
---

# Platform Operations Runbook

## First five minutes

1. Confirm incident scope, region, tenant class, release/config, and security indicator.
2. Assign owner/incident channel and record start/correlation IDs.
3. Check verified success, queue, crash, fallback, control-plane/data-store health.
4. Stop promotion and preserve evidence.
5. Apply the smallest reversible containment.

## Common symptoms

### Sessions stuck queued

Check class-specific capacity, quota, scheduler health, worker registration/leases, region policy, and autoscaler. Do not route to weaker isolation automatically.

### Sessions stuck starting

Check artifact/image pull, V8 snapshot/Chromium startup, worker grants, sandbox/network policy, filesystem/disk, and version mismatch. Drain repeatedly failing worker image.

### Native crash increase

Identify build/capability/domain cluster, disable affected capability or route eligible tasks to Chromium, preserve crash bundles, roll back if release correlated.

### Chromium crash/resource spike

Check browser version, renderer/GPU flags, context density, memory pressure, downloads/media, and worker recycle. Reduce density or roll back.

### Fallback spike

Check native regression, capability registry/config, domain/site changes, protocol behavior, and router version. Fallback protects success but cost alarm remains actionable.

### Database/Redis/object storage degradation

Protect durable writes, shed noncritical analytics/artifact work, preserve session terminal states/outbox, follow managed-service failover, and avoid treating Redis as durable truth.

### Suspected data/security issue

Follow `security/INCIDENT_RESPONSE.md`; restrict access, preserve evidence, involve security/legal, and do not paste sensitive data into normal tickets or model context.

## Recovery verification

Run synthetic sessions for native, Chromium, migration, protocol, workflow, and artifact paths; confirm queue/error/resource/SLO recovery; monitor canary; document cause and follow-up.

## Escalation

Use component ownership and severity. Production/security/legal gates require accountable humans. Agents may execute documented diagnostics and reversible mitigations within granted permissions.
