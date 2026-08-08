---
title: "Data Retention and Deletion"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define retention classes, deletion verification, legal holds, and artifact minimization."
---

# Data Retention and Deletion

## Principles

Collect less, retain briefly, separate metadata from content, make policy explicit, verify deletion, and honor legal/security constraints.

## Suggested defaults for owner approval

| Data | Suggested default |
| --- | --- |
| Session control metadata | 30 days |
| Aggregated usage/SLO data | 13 months with minimized dimensions |
| Successful detailed traces | 24 hours or sampled shorter policy |
| Error/reproduction bundles | 7 days, extend only for active support/security case |
| Screenshots/DOM/network bodies | Off by default; explicit project policy and short retention |
| Workflow definitions/versions | Until project deletion/retention policy |
| Audit metadata | 1 year or contractual/legal policy |
| Secret values | External vault policy; never artifact/log retention |

These are planning recommendations, not legal commitments.

## Deletion workflow

1. Revoke access immediately.
2. Mark deletion request with tenant/resource scope and legal-hold check.
3. Delete/tombstone transactional metadata according to integrity/idempotency needs.
4. Purge object artifacts, derived indexes, caches, and replicas within policy window.
5. Let backups expire or apply supported deletion process according to policy.
6. Record verification without retaining deleted content.

## Legal/security holds

Authorized owners may suspend deletion for defined scope/purpose/expiry. Holds are audited and not available to ordinary project roles.

## Worker ephemera

Session files, memory, cookies/storage, downloads, and secret values are destroyed on close/recycle according to isolation tier. Persistent profiles follow explicit profile retention, not worker lifecycle.

## Testing

Seed identifiable synthetic records across stores, request deletion, and verify absence/access denial/expiry. Include artifact signed URLs and search/analytics projections.
