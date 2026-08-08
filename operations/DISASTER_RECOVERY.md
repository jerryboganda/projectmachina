---
title: "Disaster Recovery"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define outage scenarios, recovery priorities, RPO/RTO targets, and final rehearsal requirements."
---

# Disaster Recovery

## Scenarios

- Regional worker-pool loss.
- Control-plane/database unavailability or corruption.
- Object-store/artifact outage.
- Credential/key compromise.
- Bad release/configuration across fleet.
- Cloud account/service degradation.
- Supply-chain compromise requiring rebuild.

## Recovery priorities

1. Protect security and prevent additional side effects/data loss.
2. Preserve/restore authorization, policy, and durable session/workflow state.
3. Stop unsafe new execution.
4. Restore control API and scheduling.
5. Restore native/Chromium capacity by isolation class.
6. Restore artifacts/analytics and lower-priority features.

## Initial engineering objectives

- RPO for transactional control data: ≤15 minutes, target lower with PITR.
- RTO for control API in a recoverable regional event: ≤4 hours initially.
- Worker capacity can be recreated from signed immutable artifacts rather than backed-up hosts.
- Artifact RPO/RTO depends on class and customer promise.

Final contractual targets require evidence and owner approval.

## Failover constraints

Do not move sessions/data across regions when residency or secret/network policy forbids it. Active page memory is ephemeral; affected sessions may terminate and require safe replay. Side-effecting workflows resume only from verified checkpoint.

## Rehearsal

M9 exercise includes restore control data, recreate worker pools, rotate/reissue grants, validate policy/auth, run native/Chromium/migration/workflow synthetic tasks, measure RPO/RTO, and test communication/rollback.

## Documentation

Maintain contacts, dependencies, infrastructure bootstrap, key/vault recovery, DNS/traffic controls, data restore, verification, customer impact decision, and return-to-normal procedure.
