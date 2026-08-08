---
title: "Build System and Reproducibility"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define the cross-language build, code generation, caching, artifacts, and reproducible release workflow."
---

# Build System and Reproducibility

## Build spine

- Cargo workspace for Rust crates/services/workers/CLI.
- CMake + Ninja for `cpp/v8-bridge` and external V8 artifact integration.
- pnpm workspace for SvelteKit, UI, TypeScript SDK, and tooling.
- Buf/protoc for protobuf; deterministic OpenAPI/JSON Schema generation.
- `just` as discoverable task facade.

## Build graph

```text
toolchain + pinned V8/Chromium artifacts
 -> C++ bridge
 -> Rust foundation/contracts
 -> native engine and workers
 -> services/protocol adapters
 -> generated SDK contracts
 -> SDKs/frontend
 -> packages/images/release manifest
```

## Code generation

Canonical sources:

- `proto/` for gRPC/events.
- command/capability schema definitions for canonical behavior.
- OpenAPI generated/validated from HTTP contract.
- JSON Schema for workflows/policies/artifacts.
- pinned CDP/BiDi definitions for adapter types.

Generated outputs include a source hash and generator version. CI fails on dirty regeneration.

## V8/Chromium artifacts

Pin exact versions and build flags. Download only from approved source with checksum/signature, or build in controlled pipeline. Cache by content hash. Runtime verifies compatible V8 snapshot/bridge build.

## Reproducibility

- Lock dependencies and base-image digests.
- Normalize timestamps/paths where possible.
- Isolate network access during final build after dependencies are fetched.
- Record compiler/linker flags and environment.
- Build release artifacts twice in independent workers and compare expected-reproducible outputs or document controlled variance.

## Build commands

```bash
just build-native
just build-services
just build-frontend
just generate
just build-all
just package
just release-artifacts VERSION=x.y.z
```

## Caching

Use content-addressed compiler/package caches with tenant/repository isolation in hosted CI. Never cache secrets, credentials, generated private artifacts, or untrusted executable output across trust boundaries.

## Release artifacts

- Native binaries and libraries by supported target.
- Server/worker/console container images.
- SDK packages.
- Helm/deployment templates.
- SBOM, provenance, signatures, checksums.
- Capability matrix and release/test reports.
