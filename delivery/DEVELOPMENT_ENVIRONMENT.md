---
title: "Development Environment"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define reproducible local, containerized, and managed environments for humans and coding agents."
---

# Development Environment

## Supported developer hosts

- Linux x86_64/arm64: primary.
- macOS arm64/x86_64: supported for most development; Linux VM/container for kernel sandbox tests.
- Windows x86_64: native target by GA; WSL2 may bootstrap early development but is not the final product experience.

## Required tool classes

- Git with worktree support.
- Rust stable toolchain pinned by `rust-toolchain.toml`, rustfmt, clippy, cargo tools as approved.
- Clang/LLVM, CMake, Ninja, Python for V8/build integration.
- Node.js LTS and pnpm pinned through project metadata.
- Protobuf/Buf and schema generators.
- Docker/Compose and optional local Kubernetes.
- PostgreSQL, Redis-compatible server, S3-compatible object store through containers.
- Browser/reference test dependencies and certificates.

Pin exact versions in M0. This document intentionally avoids hardcoding a stale future version.

## Bootstrap command

The repository should provide:

```bash
./scripts/dev/bootstrap
just doctor
just dev-up
just fast-gate
```

Bootstrap is idempotent, verifies checksums/signatures, does not install global packages without explicit notice, and supports non-interactive agent use.

## Environment profiles

- `minimal-native`: Rust/C++ engine and unit tests.
- `platform`: data services and control plane.
- `frontend`: Node/pnpm/SvelteKit and generated local API.
- `full-local`: native + Chromium workers + platform + console.
- `security`: Linux sandbox/sanitizer tools.
- `certification`: pinned full suite image/hosts and larger resource allocation.

## Secrets

Use `.env.example` with names and safe defaults only. Local secrets go in ignored files or approved local vault. Agents never print them. Production secrets are never mounted in development.

## Agent worktrees

Bootstrap creates an optional common build cache but separate source worktrees. Caches must be concurrency safe and cannot share mutable generated output directories. Each task records environment/toolchain hash.

## Doctor checks

Verify architecture, disk/memory, toolchain versions, V8/Chromium artifacts, ports, containers, certificates, data services, sandbox support, and clean generated contracts. Output actionable remediation and machine-readable result.
