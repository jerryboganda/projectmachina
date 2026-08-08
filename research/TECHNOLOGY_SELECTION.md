---
title: "Technology Selection and Alternatives"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Engineering"
purpose: "Record the recommended implementation stack, alternatives, constraints, and evidence required before substitutions."
---

# Technology Selection and Alternatives

## Selection principles

Project Machina chooses technology by total task throughput, correctness, security, operability, ecosystem maturity, and agentic-development suitability. A microbenchmark alone is not sufficient. The selected stack must also support generated contracts, deterministic builds, narrow ownership boundaries, and independent parallel work.

## Recommended stack

| Layer | Baseline selection | Why it fits | Reconsider when |
| --- | --- | --- | --- |
| Native browser core | Rust stable, pinned by `rust-toolchain.toml` | Memory safety, strong async and systems ecosystem, explicit ownership, good FFI control | Evidence shows a blocking V8/ABI or platform constraint |
| JavaScript engine | V8 | Mature embeddable JavaScript/Wasm engine; browser compatibility target; isolate and snapshot mechanisms | A replacement meets compatibility, licensing, startup, and memory gates |
| V8 boundary | Small C++20 shim with generated C ABI/Rust bindings | Contains ABI volatility and keeps C++ ownership localized | A maintained safe binding meets all requirements |
| HTML/tokenization | Standards-oriented streaming parser components plus Project Machina tree builder | Enables progressive parse and compact DOM | WPT evidence proves another parser is superior |
| DOM/runtime | Custom arena-backed Rust representation | Optimizes handles, wrappers, revisions, reset, extraction, and semantic deltas | Prototype fails compatibility or memory gates |
| Async runtime | Tokio-compatible design, abstracted at component boundary | Strong Rust ecosystem and cancellation support | Benchmarks show scheduler cost is material and replacement is feasible |
| HTTP/network | Rust HTTP/TLS stack with HTTP/1.1 and HTTP/2 first; HTTP/3 later | Async pooling and controllable policy surface | Compatibility or proxy requirements demand a different stack |
| Browser fallback | Pinned Chromium worker images driven through a controlled adapter | Maximum practical compatibility and visual fidelity | Another renderer materially improves target coverage/cost |
| Public API | HTTP/JSON plus gRPC/event streams | Broad client access plus typed high-throughput control | Product usage proves one surface unnecessary |
| Automation protocols | CDP, WebDriver BiDi, MCP | Existing Playwright/Puppeteer ecosystem, standards direction, and agent tooling | Version support policy changes |
| Contract definitions | Protobuf/OpenAPI/JSON Schema generated from one model | Prevents drift between adapters and SDKs | Generator creates semantic loss |
| Durable control state | PostgreSQL | Transactions, relational constraints, migrations, operational maturity | Scale evidence requires a complementary store |
| Ephemeral coordination | Redis-compatible service | Leases, queues, rate counters, short-lived session coordination | Correctness needs stronger durability for a specific record |
| Artifacts | S3-compatible object storage | Traces, recordings, profiles, reproduction bundles, reports | Deployment target provides an equivalent abstraction |
| Console | Svelte 5 + SvelteKit + TypeScript | Compiler-oriented lean client output and complete app framework | Measured product constraints favor an even smaller isolated widget |
| Styling | Design tokens + semantic CSS; minimal dependency surface | Predictable bundle and accessibility | A component system meets strict budgets and reduces total cost |
| Observability | OpenTelemetry-compatible traces, metrics, and logs | Vendor-neutral correlation across control and data planes | No replacement without an export/migration path |
| Local orchestration | Docker Compose | Fast reproducible developer environment | Native-only bootstrap becomes materially faster and equivalent |
| Production orchestration | Kubernetes | Declarative rollout, isolation, scheduling, autoscaling ecosystem | Deployment scale does not justify it or customer requires alternative |
| CI | GitHub Actions baseline with portable scripts | Repository-native automation and review integration | Owner selects another CI provider |
| Documentation | Markdown in repository | Tool-neutral, diffable, loadable by coding agents | Generated site may supplement but not replace it |

## Frontend decision

Use Svelte 5 and SvelteKit for the authenticated control console and documentation shell. Svelte compiles components to optimized JavaScript, while SvelteKit provides routing, server rendering, data loading, form handling, adapters, and mixed prerender/dynamic rendering. This avoids rebuilding application infrastructure around plain Svelte.

Deployment modes:

- prerender public documentation and static marketing/help pages;
- server-render authenticated shell and authorization-sensitive routes;
- stream trace/session updates over event transport;
- lazy-load heavy trace, graph, and editor modules;
- use plain Svelte + Vite only for a genuinely standalone embedded widget;
- do not introduce a second frontend framework without an ADR.

Required frontend budgets are defined in `architecture/FRONTEND.md`; the choice remains subject to measured bundle, interaction, accessibility, and memory results rather than framework reputation.

## Rust and V8 boundary rules

- V8 objects never cross the FFI as raw long-lived pointers without an explicit owner handle.
- Every handle records isolate identity and generation.
- One component owns platform initialization and shutdown.
- Isolate entry, locker/thread rules, exception translation, microtask checkpoints, cancellation, and memory limits are centralized.
- Generated bindings are reproducible and checked against the pinned V8 revision.
- C++ exceptions do not cross the ABI.
- Rust panics do not cross the ABI.
- All unsafe Rust has a documented invariant and focused test.
- Snapshot format and V8 revision are version-coupled artifacts.

## Data-system boundaries

PostgreSQL owns organizations, identities, projects, API credentials metadata, policies, workflow definitions, immutable workflow versions, task/session metadata, billing/metering records, approvals, and audit indexes.

Redis-compatible storage owns only replaceable or reconstructable coordination state: leases, queues, rate windows, transient presence, short-lived routing hints, and deduplication tokens. A Redis loss must not erase authoritative product state.

Object storage owns large immutable artifacts: trace segments, protocol logs, benchmark data, recordings, heap/CPU profiles, crash bundles, WPT reports, release evidence, and exports. Database records reference artifacts by content hash and retention class.

## Protocol selection rules

### CDP

Support a pinned, certified schema subset. The CDP tip-of-tree changes frequently and does not guarantee backward compatibility for new capabilities, so generated adapters and explicit version ranges are mandatory.

### WebDriver BiDi

Treat BiDi as a first-class public protocol because it provides bidirectional command/event behavior designed for remote control of user agents. Implement modules incrementally behind the same command bus.

### MCP

Expose safe browser and workflow tools with capability negotiation, strict schemas, deadlines, cancellation, progress, auditing, and least privilege. Pin a protocol revision and update through compatibility tasks rather than silently following a draft.

## Technology-change process

A substitution needs an ADR containing:

1. the measured problem with the current choice;
2. at least two feasible alternatives;
3. compatibility and migration impact;
4. security and licensing review;
5. build/release impact;
6. agentic-development impact;
7. benchmark and acceptance plan;
8. rollback strategy.

Agents may update patch/minor dependencies under the version policy. They may not replace a foundational technology solely because a model prefers a different library.

## Required proof before GA

- reproducible Linux x86_64 and arm64 builds;
- documented macOS developer build and platform limitations;
- Windows strategy with explicit native/WSL support status;
- dependency SBOM and license report;
- V8 revision/snapshot reproducibility;
- API/client compatibility matrix;
- console performance and accessibility evidence;
- database migration/restore drill;
- telemetry export and redaction verification;
- Kubernetes rollout, rollback, and autoscaling evidence;
- clean local bootstrap from the documented toolchain.

## Official references

- V8 embedding: https://v8.dev/docs/embed
- V8 build: https://v8.dev/docs/build
- Svelte overview: https://svelte.dev/docs/svelte/overview
- SvelteKit introduction: https://svelte.dev/docs/kit/introduction
- SvelteKit static generation: https://svelte.dev/docs/kit/adapter-static
- WebDriver BiDi: https://www.w3.org/TR/webdriver-bidi/
- Chrome DevTools Protocol: https://chromedevtools.github.io/devtools-protocol/
- MCP specification: https://modelcontextprotocol.io/specification/2026-07-28
- OpenTelemetry: https://opentelemetry.io/docs/
- Kubernetes: https://kubernetes.io/docs/home/
- PostgreSQL: https://www.postgresql.org/docs/current/
