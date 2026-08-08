---
title: "Rust Core and Narrow C++ V8 Bridge"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture Review"
purpose: "Record an accepted architecture decision for Project Machina."
---

# ADR-002 — Rust Core and Narrow C++ V8 Bridge

## Status
Accepted.

## Context
The runtime handles hostile input, asynchronous state, complex lifetimes, and high concurrency. V8 has a C++ embedding API.

## Decision
Implement core systems in Rust and isolate V8-specific C++ behind a small audited C ABI with a safe Rust facade. Use V8 startup snapshots.

## Consequences

- Strong memory/concurrency safety foundation.
- FFI and build integration remain high-risk and require sanitizers/fuzzing.
- V8 version upgrades are controlled, pinned events.
