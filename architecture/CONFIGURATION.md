---
title: "Configuration and Feature Policy"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define precedence, validation, dynamic settings, secrets, feature flags, and safe defaults."
---

# Configuration and Feature Policy

## Sources and precedence

Lowest to highest:

1. compiled safe defaults;
2. versioned deployment configuration;
3. organization policy;
4. project policy;
5. session request within allowed bounds;
6. workflow step override within policy;
7. emergency administrative restriction only.

A lower-trust source cannot weaken an upper-level security restriction.

## Configuration classes

- static build/runtime: engine paths, supported protocols, storage drivers;
- deployment: region, endpoints, pool sizes, feature rollout;
- tenant policy: quotas, egress, retention, isolation, fallback;
- session: fidelity, deadlines, viewport, locale, proxy reference;
- workflow: variables, approvals, retries, recovery;
- emergency: domain block, feature kill, worker drain.

## Validation

All configuration is schema-versioned and validated at ingestion. Unknown security-relevant fields fail closed. Return effective configuration hash and relevant normalized values without revealing secrets.

## Secrets

Configuration contains secret references, never plaintext. Runtime resolution is scoped, authenticated, audited, expiring, and redacted.

## Feature flags

Every flag records:

- stable ID and owner;
- description and affected components;
- default and scope;
- security/compatibility impact;
- telemetry;
- rollout and rollback;
- expiry/removal condition.

Flags do not create undocumented public behavior. Capability responses include flag-disabled status.

## Dynamic reload

Only documented settings reload dynamically. Security tightening may apply immediately to new operations; loosening requires authorization. Engine/toolchain changes require worker replacement. Configuration version is attached to sessions for reproducibility.

## Safe defaults

- dedicated-process isolation for untrusted managed workloads;
- deny local/private/metadata network ranges;
- block downloads and file uploads unless allowed;
- prefer-native with explicit fallback for target workloads;
- bounded deadlines, requests, bytes, memory, frames, workers, and artifacts;
- no page-content trace capture;
- robots/rate-policy respectful behavior for crawler profiles;
- no production secret exposure to development environments.
