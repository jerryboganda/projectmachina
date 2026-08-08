---
title: "Toolchain, Runtime, and Dependency Policy"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Keep fast-moving browser, V8, Rust, Svelte, protocol, and cloud dependencies controlled and supported."
---

# Version and Dependency Policy

## Pinning

Pin Rust toolchain, Node/pnpm, Clang/CMake, V8, Chromium, protocol schemas, container images, CI actions, and application dependencies. Lockfiles are committed. Release artifacts record all versions and flags.

## Update classes

- Emergency security: expedited branch, focused security/regression, canary, release.
- Routine patch/minor: automated proposal, fast gate plus affected scheduled suites.
- Major/toolchain/browser: planned task, compatibility/performance/security review, migration and rollback.

## Dependency admission

Document purpose, alternatives, maintenance, license, source/provenance, size/performance, security history, transitive graph, and removal cost. Avoid dependency for trivial code when risk outweighs value; avoid homemade crypto/protocol primitives.

## Browser/engine cadence

V8 and Chromium updates may affect ABI, snapshots, CDP, behavior, memory, and security. Upgrade in isolated work, regenerate artifacts/schemas, run focused compatibility/WPT/differential/performance/security, then canary.

## Frontend cadence

Use supported Svelte/SvelteKit versions and official migration tooling/docs. Keep UI dependencies limited and review bundle/runtime effects. Do not upgrade core framework concurrently with unrelated feature work.

## Support windows

Publish supported server, SDK, client, OS/architecture, database, Kubernetes, and protocol versions. Remove support only through deprecation and evidence.

## Bots

Dependency bots may open grouped low-risk updates but must not auto-merge V8, Chromium, auth, crypto, sandbox, build, protocol schema, or major framework updates without owner review.
