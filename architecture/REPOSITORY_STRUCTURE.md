---
title: "Repository Structure"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define a monorepo layout that supports independent agents, generated contracts, and isolated component ownership."
---

# Repository Structure

## Recommended monorepo

```text
/
├── AGENTS.md
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── CMakeLists.txt
├── package.json
├── pnpm-workspace.yaml
├── justfile
├── buf.yaml
├── docs/                         # this documentation pack
├── adr/                          # accepted runtime ADR mirror if desired
├── crates/
│   ├── command-model/            # canonical typed commands/events/errors
│   ├── command-bus/              # single internal command execution route
│   ├── session/                  # lifecycle and policy primitives
│   ├── capability/               # registry and routing decisions
│   ├── native-core/              # engine composition
│   ├── html/                     # tokenizer/tree builder
│   ├── dom/                      # nodes, mutation, selectors, ranges
│   ├── events/                   # event targets, dispatch, focus/input
│   ├── navigation/               # lifecycle/history/waits
│   ├── runtime-v8/               # safe Rust facade
│   ├── event-loop/               # tasks/microtasks/timers
│   ├── network/                  # HTTP, fetch, cache, policy integration
│   ├── storage/                  # cookies/web storage/persistent profile
│   ├── semantic/                 # roles/names/visibility/interactability
│   ├── extraction/               # markdown/schema/metadata outputs
│   ├── state-bridge/             # transferable state and action replay
│   ├── scheduler/                # local worker scheduler primitives
│   ├── protocol-cdp/
│   ├── protocol-bidi/
│   ├── protocol-mcp/
│   ├── protocol-http/
│   ├── telemetry/
│   └── security-policy/
├── cpp/
│   └── v8-bridge/                # narrow C ABI/C++ V8 ownership boundary
├── services/
│   ├── api-gateway/
│   ├── session-control/
│   ├── worker-native/
│   ├── worker-chromium/
│   ├── workflow-service/
│   ├── artifact-service/
│   ├── usage-service/
│   └── admin-ops/
├── apps/
│   ├── console/                  # SvelteKit
│   ├── docs-site/                # SvelteKit static/prerendered
│   └── playground/               # optional isolated developer UI
├── packages/
│   ├── contracts-ts/             # generated TypeScript types/client
│   ├── ui/                       # Svelte design system
│   ├── sdk-typescript/
│   ├── sdk-python/
│   ├── sdk-go/
│   ├── sdk-java/
│   └── sdk-rust/
├── proto/                        # canonical protobuf/event schemas
├── openapi/                      # generated/validated HTTP contract
├── schemas/                      # JSON Schema/workflow/capability formats
├── tests/
│   ├── fixtures/
│   ├── contract/
│   ├── integration/
│   ├── differential/
│   ├── wpt/
│   ├── conformance/
│   ├── security/
│   ├── performance/
│   ├── reliability/
│   └── e2e/
├── benchmarks/
│   ├── corpus/
│   ├── harness/
│   └── reports/
├── deploy/
│   ├── compose/
│   ├── kubernetes/
│   ├── helm/
│   └── terraform/
├── scripts/
│   ├── agent/
│   ├── build/
│   ├── test/
│   ├── release/
│   └── dev/
├── .agent-state/                 # claims/evidence projection; sensitive output ignored
├── .github/
├── .claude/
├── .agents/
└── antigravity/
```

## Ownership boundaries

- Each crate/service/package has a `CODEOWNERS` group and optional nested `AGENTS.md` for stricter rules.
- `command-model`, `proto`, `schemas`, root lockfiles, and build/CI files are serialized shared-contract scopes.
- Protocol adapters depend inward on the command model; they do not import engine implementation details.
- Frontend uses generated contracts rather than handwritten duplicate API types.
- Tests live close to units for fast checks and under `tests/` for cross-component suites.

## Dependency direction

```text
foundation utilities
  -> command/session/capability contracts
  -> engine components
  -> worker composition
  -> services/protocol adapters
  -> SDKs and frontend
```

No lower layer imports an API gateway, UI, cloud service, or protocol adapter.

## Build policy

- Rust workspace is the native implementation spine.
- CMake builds the V8 bridge as a versioned artifact consumed by Rust bindings.
- pnpm manages frontend and TypeScript packages.
- Buf/protobuf and OpenAPI generation are deterministic.
- `just` exposes human- and agent-friendly commands without hiding underlying tool output.
- Toolchains and dependencies are pinned; update bots open isolated PRs.

## Agent-state policy

Commit human-readable state projections and task evidence summaries. Ignore raw logs, local paths, secrets, large traces, build outputs, and ephemeral claim locks according to the implementation established in M0.
