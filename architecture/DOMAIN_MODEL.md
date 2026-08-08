---
title: "Domain Model"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define durable and runtime entities, identifiers, ownership, and lifecycle relationships."
---

# Domain Model

## Tenancy hierarchy

```text
Organization
  -> Project
      -> API credential / member / policy
      -> Session
          -> Browser context
              -> Page/frame/worker
          -> Command and event stream
          -> Trace/reproduction artifacts
      -> Workflow
          -> Version
          -> Schedule
          -> Run
              -> Approval request
      -> Usage and audit records
```

## Core entities

### Organization

Billing/security boundary. Owns projects, members, roles, quotas, retention, and region policies.

### Project

Operational boundary for API keys, session defaults, origins, proxies, workflows, usage, and access control.

### Session

Top-level scheduled browser execution with immutable identity and versioned effective policy. States: `requested`, `queued`, `starting`, `ready`, `migrating`, `closing`, `closed`, `failed`, `expired`.

### BrowserContext

Isolation of cookies, storage, permissions, cache, and pages. Native and Chromium implementations expose the same conceptual identity, but support level is capability-versioned.

### Page

Navigation and document lifecycle resource. It owns frames, semantic revision, action history, and page-scoped events.

### Command

Immutable requested operation with command ID, correlation ID, deadline, idempotency class, capability needs, and typed payload.

### CommandOutcome

Success or typed failure plus engine, timings, resource usage, verification, approximation/fallback, and trace references.

### CapabilitySnapshot

Versioned support state by engine/build/config: `native`, `native-limited`, `chromium`, `unsupported`, `disabled-by-policy`, or `experimental`, with evidence reference.

### Policy

Versioned rules for fidelity, fallback, isolation, egress, downloads, secrets, approvals, quotas, retention, and observability.

### TransferBundle

Versioned, encrypted-in-transit bundle of allowed cookies, storage, headers, proxy/context configuration, current URL, navigation metadata, and action log. It has redaction and same-origin constraints.

### Workflow and WorkflowVersion

Immutable executable definition per version. The logical workflow points to active/approved versions. A run references the exact version and effective policy.

### Artifact

Trace, recording, benchmark result, crash bundle, export, or report with classification, tenant owner, retention, hash, encryption, and access policy.

## Identifier requirements

- Globally unique, non-sequential public IDs.
- Tenant/resource type visible only if it does not leak sensitive information.
- IDs immutable and never reused.
- Event ordering uses per-session monotonically increasing sequence plus timestamp.
- Correlation and causation IDs connect commands, engine events, migrations, and workflow steps.

## Deletion model

Use immediate authorization revocation, soft deletion for recoverable control metadata where policy permits, asynchronous verified purge of artifacts and derived data, and tombstones only where needed for idempotency/audit. Retention and legal holds are explicit.
