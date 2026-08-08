---
title: "Security Review Checklist"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Give agents and reviewers a practical gate for code, protocols, runtime, data, and deployment changes."
---

# Security Review Checklist

## Every security-relevant task

- [ ] Threats and trust boundaries identified.
- [ ] Inputs typed, normalized, size/time bounded, and treated as untrusted.
- [ ] Authorization enforced server-side for each resource/event/artifact.
- [ ] Tenant IDs and scopes cannot be supplied to bypass ownership.
- [ ] Secrets remain references and are redacted in all outputs.
- [ ] Cancellation and resource ceilings exist.
- [ ] Errors do not reveal sensitive internals.
- [ ] Logs/metrics avoid high-cardinality or page-sensitive values.
- [ ] Failure is closed or explicitly safe.
- [ ] Focused negative tests added and run now.

## Native runtime/FFI

- [ ] Ownership, lifetime, thread, exception, and buffer invariants documented.
- [ ] No unchecked raw pointer/length from page input.
- [ ] Stale handles fail safely.
- [ ] Recursion/allocation/task queues bounded.
- [ ] Fuzz/sanitizer target updated.

## Network

- [ ] URL/scheme/host/port normalization.
- [ ] DNS/IP/redirect/rebinding checks.
- [ ] Proxy credentials and pool identities isolated.
- [ ] Requests/bytes/connections/time bounded.
- [ ] Private/metadata destinations denied by default.

## Sandbox/deployment

- [ ] Non-root, minimal capabilities, read-only root.
- [ ] No host socket/cloud credential/general secret mount.
- [ ] cgroup/system-call/network policy.
- [ ] Ephemeral filesystem and download/upload handling.
- [ ] Cross-session reset/recycle behavior.

## Agent/workflow

- [ ] Page prompt injection cannot modify policy/tools.
- [ ] Side effects classified and replay-safe.
- [ ] High-impact approval policy evaluated.
- [ ] Workflow expressions cannot execute arbitrary host code.
- [ ] Recording excludes secret values.

## Release

- [ ] Dependency/license/SBOM/provenance checks.
- [ ] Critical/high findings resolved or authorized.
- [ ] Sandbox, egress, tenant, secret-canary, and rollback tests passed.
- [ ] Security owner reviewed material changes.
