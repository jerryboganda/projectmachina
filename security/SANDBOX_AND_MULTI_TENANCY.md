---
title: "Sandbox and Multi-Tenancy"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Specify isolation controls, worker identities, filesystem, syscalls, resources, resets, and tenant tests."
---

# Sandbox and Multi-Tenancy

## Baseline worker controls

- Dedicated unprivileged UID/GID or container user.
- No privileged mode, host PID/network namespace, Docker socket, or cloud credential mount.
- Read-only root filesystem and explicit ephemeral writable paths.
- Minimal capabilities; drop all then add none unless proven required.
- Seccomp/system-call policy and LSM profile where supported.
- Cgroup CPU, memory, PIDs, I/O, and process-count limits.
- Network namespace with enforced egress policy.
- No access to control-plane database or general secret store.
- Signed expiring session execution grant.

## Isolation tiers

### Shared-performance

Logical separation inside one process is permitted only for approved workload classes. Requirements include separate V8 contexts/isolate policy, DOM/storage/network state, quotas, generational handles, complete reset, and cross-session differential security tests. A crash may affect co-resident sessions; this is disclosed.

### Dedicated-process

One session/tenant group per process under OS sandbox. Default for untrusted managed workloads. A worker crash affects only that process boundary.

### Hardened

Container or microVM per session/group with dedicated kernel/network/filesystem boundary, stronger image and attestation policy, and no shared mutable profile. Enterprise/high-risk use.

## Filesystem

- Per-session ephemeral directory with random identifier and strict permissions.
- Uploads enter through controlled file handles and size/type policy.
- Downloads go to isolated storage, are scanned/policy checked, and exported as artifacts—not host paths.
- Resolve paths canonically and prevent traversal/symlink/device access.
- Wipe/release storage after retention/close.

## Reset invariants

Before worker reuse, clear or recreate:

- V8 isolate/context and native wrappers;
- DOM, listeners, timers, workers, WebSockets;
- cookies/storage/cache unless explicit persistent profile;
- DNS/TLS/connection state if identity/policy incompatibility exists;
- temporary files/downloads/uploads;
- secrets and proxy credentials;
- telemetry buffers and tenant labels.

Prefer process recycle over complex reset for untrusted tiers.

## Tenant isolation tests

- Attempt resource ID access across tenants.
- Seed unique cookies/storage/secrets and verify absence in next session.
- Trigger crash/timeout/heap exhaustion and verify other tenant outcome.
- Exercise event streams and artifact access cross-tenant.
- Test connection/proxy pool identity boundaries.
- Test shared-cache key partitioning.
- Test stale handle and session ID reuse.

## Host hardening

Minimal patched host, immutable images, restricted SSH/admin, workload identity, runtime detection, kernel security updates, node-pool separation, and emergency drain/reimage.
