---
title: "Release Strategy"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define maturity channels, semantic versioning, release evidence, canary, rollback, and public claims."
---

# Release Strategy

## Channels

- `dev`: continuous internal builds.
- `alpha`: APIs/capabilities may change; target users only.
- `beta`: production-intended subset with explicit limitations and support policy.
- `rc`: frozen candidate under M9 certification.
- `stable/GA`: approved compatibility/security/operations commitments.

## Versioning

Use semantic versioning for product API/package surfaces, with separate engine/capability build metadata. Pre-1.0 breaking changes still require migration notes and explicit client matrix.

## Release contents

- Immutable binaries/images/packages and checksums/signatures.
- SBOM and provenance.
- Release notes and upgrade/rollback guide.
- Database/schema migrations.
- Capability and protocol/client matrix.
- Known limitations and fixed security notices as appropriate.
- Test/certification and benchmark references.
- SLO/readiness and operations notes.

## Canary

Start with internal/synthetic and low-risk tenant traffic. Compare verified success, errors, crashes, fallback, latency, memory, queueing, and security signals. Promote through staged percentages/regions. Halt automatically on predefined thresholds.

## Rollback

- Keep previous compatible artifacts and configuration.
- Use backward-compatible expand/migrate/contract database changes.
- Drain workers and stop new sessions on bad build.
- Preserve workflow/state format read compatibility or migration rollback.
- Record rollback reason and run focused regression before repromotion.

## Public claims

Only state native/hybrid performance and compatibility supported by final evidence. Name versions, workloads, fidelity, success, and limitations. Independent approval is required.

## Release authorization

Release lead assembles evidence. Security, legal/license, platform, independent claim reviewer, and product owner approve their gates. An autonomous agent may prepare and verify but cannot grant accountable approval.
