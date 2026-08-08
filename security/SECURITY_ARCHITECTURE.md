---
title: "Security Architecture"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define defense-in-depth controls for hostile pages, multi-tenancy, APIs, workers, data, secrets, and operations."
---

# Security Architecture

## Security objectives

- Prevent hostile page code from escaping its assigned isolation boundary.
- Prevent tenant-to-tenant data or capability access.
- Prevent browser workloads from reaching prohibited internal/local networks.
- Prevent secrets and sensitive page content from leaking through code, logs, traces, recordings, artifacts, or agent context.
- Ensure high-impact actions are policy controlled and auditable.
- Preserve integrity of builds, dependencies, releases, and capability claims.
- Detect, contain, investigate, and recover from security events.

## Assumptions

- Every fetched byte, script, document, frame, worker, WebSocket message, download, and page-generated string is hostile.
- Agent/model output is untrusted input.
- Page content may contain prompt injection intended to change agent behavior.
- Protocol clients may be malformed or malicious.
- Tenant credentials may be stolen; scope and anomaly controls still matter.
- Native engine and V8/Chromium can contain vulnerabilities; isolation must not rely solely on memory safety.

## Defense layers

1. **Identity:** authenticated users/services, short-lived scoped credentials, rotation.
2. **Authorization:** organization/project/resource policy on every server-side operation.
3. **Input validation:** typed schemas, limits, normalization, no unsafe deserialization.
4. **Isolation:** shared/dedicated/hardened tiers, process/container/microVM controls.
5. **Network control:** DNS/IP/scheme/port/redirect/proxy policy and egress observation.
6. **Runtime limits:** CPU, memory, requests, bytes, frames, workers, deadlines, filesystem.
7. **Secrets:** opaque references, moment-of-use resolution, redaction and audit.
8. **Data protection:** encryption, tenant keys/policy, retention/minimization, access logging.
9. **Supply chain:** pinned dependencies, provenance, SBOM, signed artifacts, scanning.
10. **Detection/response:** audit, anomaly, canary secrets, incident runbooks, circuit breakers.

## Authentication

- Human console sessions use approved identity provider or secure local identity for self-hosted mode.
- Service/API credentials are project-scoped, hashed at rest where appropriate, rotatable, and optionally short-lived.
- Worker execution uses signed, expiring grants rather than broad control-plane credentials.
- Administrative access requires stronger authentication and just-in-time elevation.

## Authorization

Use centralized policy evaluation with resource hierarchy and explicit scopes. Defend against insecure direct object references by always checking tenant ownership after lookup. Event streams and artifacts are authorization-checked independently.

## Page/runtime separation

Workers run unprivileged, with minimal filesystem, no host socket, bounded syscalls, and controlled network namespace. Page code cannot call control-plane APIs or secret vault directly. Native host functions validate handles, origin, context, and policy.

## Agent separation

System/developer policy, workflow definition, user request, and page observations remain separate labeled channels. Page text never grants permissions, changes tool configuration, reveals secrets, or authorizes high-impact action.

## Security release gates

- Threat model updated for new trust boundaries.
- Security-sensitive fast checks passed.
- No critical/high unresolved vulnerability without authorized acceptance.
- Dependency/SBOM/provenance clean according to policy.
- Sandbox/egress/tenant tests pass in M9.
- Incident, rollback, secret rotation, and emergency block are operational.
