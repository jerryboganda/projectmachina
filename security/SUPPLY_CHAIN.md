---
title: "Software Supply Chain Security"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Protect source, dependencies, AI-generated changes, builds, artifacts, containers, and releases."
---

# Software Supply Chain Security

## Source controls

- Protected default and release branches.
- Required review and signed/verified commits or equivalent identity policy for releases.
- CODEOWNERS for security, V8 bridge, sandbox, protocols, build, release, and dependency files.
- Secret scanning, push protection, dependency review, static analysis, and artifact retention.
- AI-generated code is reviewed under the same standard; tool identity is recorded where useful but is not trusted provenance by itself.

## Dependencies

- Lock all language/package dependencies.
- Pin V8/Chromium/toolchain/container base versions.
- Record license, source, checksum/signature, maintainer health, and purpose in SBOM/provenance.
- Prefer small, maintained dependencies; avoid duplicate foundational libraries without review.
- New runtime/parser/crypto/network/sandbox dependencies require security and license review.

## Build

- Hermetic/reproducible build goals.
- Isolated ephemeral CI workers with least privilege.
- No untrusted PR gets production/signing secrets.
- Build scripts and code generators are treated as executable dependencies.
- Produce SBOM, provenance attestation, checksums, and signatures.
- Compare reproducibility for release-critical artifacts where feasible.

## Containers

Minimal base, non-root, read-only root, no package manager in final image, explicit entrypoint, health checks, pinned digest, vulnerability scan, signed image, and deployment policy verification.

## Vulnerability response

Classify reachability and exposure, patch/upgrade, run focused regression/security checks, canary, roll out, and disclose according to policy. V8/Chromium security updates use expedited handling.

## AI coding controls

- Agents may not add dependencies without documenting purpose, alternatives, license, and security impact.
- Agents may not disable scanners or lower policy thresholds to merge.
- Generated code must have tests and ownership.
- Prompts/logs sent to external services must not include secrets or restricted source/data beyond approved policy.

## Release evidence

Release manifest includes source commit, toolchains, dependencies, SBOM, provenance, signatures, test campaign, security findings, capability matrix, migration/rollback, and approver.
