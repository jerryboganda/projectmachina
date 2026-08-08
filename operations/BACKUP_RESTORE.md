---
title: "Backup and Restore"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define protected backups, restore validation, retention, and operational procedures for durable project data and artifacts."
---

# Backup and Restore

## Protected data

- PostgreSQL control-plane data.
- Workflow definitions/versions/approvals/run metadata.
- Policies, capability registry, audit metadata, usage reconciliation.
- Object-storage artifacts according to classification/retention.
- Deployment configuration and IaC source through Git/release artifacts.
- Secret values remain in the external vault's own backup policy; Project Machina stores references/metadata.

Redis-compatible ephemeral coordination is not a primary backup source.

## Backup policy

Use encrypted automated database snapshots plus point-in-time logs where promised, object versioning/replication according to artifact class, separate credentials, least privilege, retention tiers, and tamper/ransomware-resistant copies where required.

## Restore procedure

1. Authorize and select recovery point/environment.
2. Create isolated restore destination.
3. Verify backup integrity and encryption access.
4. Restore database and object metadata/artifacts consistently.
5. Apply supported migrations only after base validation.
6. Run integrity checks, tenant counts, workflow/capability/audit checks.
7. Run synthetic sessions without contacting prohibited external destinations.
8. Document measured RPO/RTO and discrepancies.
9. Promote/fail over only with approval.

## Testing

Automated restore smoke at regular cadence and full rehearsal during M9. A backup is not considered valid until restored and verified.

## Security

Backup access is more restricted than ordinary read access, audited, and never exposed to worker/page processes. Restored non-production data is minimized/masked and access controlled.
