---
title: "Lightpanda Baseline and Differentiation Analysis"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Product, Architecture, Legal, and Performance"
purpose: "Document the verified Lightpanda baseline, the capabilities worth preserving, and the evidence-based opportunities for Project Machina to differentiate."
---

# Lightpanda Baseline and Differentiation Analysis

## Research posture

This is a competitive and architectural baseline, not a reverse-engineering specification. Project Machina is designed as an independent clean-room implementation. Engineers may study public behavior, public documentation, standards, and published protocols, but must not copy protected implementation details or source code into the clean-room core.

The comparison is intentionally evidence-bound. A capability is treated as verified only when supported by an official Lightpanda source or a reproducible test. Vendor performance figures are recorded as vendor claims until Project Machina's benchmark harness reproduces them under equivalent conditions.

## Verified Lightpanda strengths

Lightpanda publicly describes itself as a machine-oriented browser built from scratch for headless usage, with JavaScript execution, Web API support, CDP connectivity for Playwright and Puppeteer, built-in agent behavior, reproducible scripts, low-memory positioning, and fast startup. Its documentation also exposes page content as post-JavaScript HTML, Markdown, and semantic/accessibility-tree forms.

PandaScript is strategically important: it moves repeated browser actions into an in-browser JavaScript program using native primitives, eliminating the need for an LLM on replay and avoiding repeated CDP serialization. Project Machina must preserve this product advantage in its own independently designed deterministic workflow runtime.

The public repository declares AGPL-3.0-only as its default license. That makes a fork a possible technical path, but it does not match the recommended clean-room, permissive-license target for Project Machina without a separate legal and commercial decision.

## Project Machina's target advantages

| Area | Lightpanda baseline worth retaining | Project Machina target advantage | Required evidence |
| --- | --- | --- | --- |
| Machine-native execution | Native JavaScript/DOM automation without a conventional visible browser | Compact Rust engine, typed command bus, task-aware fidelity, and V8 snapshot/warm-pool optimization | Native benchmark and memory profile |
| Compatibility | CDP integrations and selected browser APIs | First-class CDP, WebDriver BiDi, MCP, HTTP/gRPC, SDKs, and automatic Chromium fallback | Versioned protocol conformance matrix |
| Unsupported behavior | Evolving implementation surface | Explicit capability negotiation, typed errors, and no silent no-op | Negative tests for every advertised command |
| Visual fidelity | Machine-focused path does not need traditional rendering for every task | Full Chromium rendering only when screenshots, PDF, media, canvas, WebGL, or difficult layout require it | Router/fallback decision traces |
| Agent workflows | Natural-language task to deterministic PandaScript replay | Typed workflow IR, semantic selectors, verification, secret references, repair policy, and optional LLM recovery | Replay success and token-cost metrics |
| Multi-tenancy | High-density browser execution | Three isolation tiers: shared, dedicated process, and hardened microVM/container | Tenant escape tests and resource isolation |
| State continuity | Browser sessions and automation | Explicit state bridge for cookies, storage, headers, proxy, locale, action log, and migration | Native-to-Chromium migration corpus |
| Observability | Browser-level output and diagnostics | End-to-end OpenTelemetry-compatible traces, revision IDs, fallback reasons, and redacted reproduction bundles | Incident drill and trace completeness audit |
| Frontend/operations | Not the primary public product emphasis | Lightweight SvelteKit console for sessions, workflows, traces, capability evidence, fleet, cost, and approvals | Accessibility and performance budgets |
| Windows support | Public instructions may use WSL for some workflows | Native, tested Windows/WSL strategy after Linux/macOS core stability | Certified build and smoke matrix |
| Safety | Robots option and guidance | Default safety policies, rate limits, egress policy, approvals, auditability, and abuse controls | Security acceptance campaign |
| Benchmark integrity | Published speed and memory positioning | Fair, reproducible, success-rate-adjusted benchmark with raw artifacts | Independent rerun instructions |

## Do not compete on the wrong metric

A single `goto()` time is not the product objective. The useful unit is a successfully verified task at a known fidelity and isolation level. Project Machina therefore optimizes and reports:

```text
successful verified tasks
-------------------------
CPU seconds + memory GB-seconds + retry cost + fallback cost + LLM token cost
```

Every public comparison must disclose:

- hardware and operating system;
- process and isolation topology;
- cache and network state;
- wait condition;
- resource policy;
- page/task corpus;
- retry and timeout rules;
- success definition;
- native/fallback split;
- p50, p95, and p99 latency;
- CPU, memory, crash, and failure rates.

## Architectural gaps Project Machina intentionally closes

These are not claims that Lightpanda lacks a feature in every version. They are Project Machina design requirements that create a broader, more robust product contract.

### One semantic core behind every adapter

HTTP, gRPC, CDP, BiDi, MCP, SDK, CLI, and deterministic workflows all invoke the same typed commands and lifecycle state machine. Adapters translate; they do not reimplement navigation or DOM behavior.

### Capability-aware automatic fallback

The router predicts and observes required capabilities. It selects native execution when safe and migrates or restarts in Chromium when full rendering or an unimplemented API is required. The response reports the engine, reason, cost class, migration result, and unsupported capabilities.

### State migration as a first-class contract

Cookies, local/session storage, selected IndexedDB state, request headers, proxy, locale, timezone, permissions, action log, and replay checkpoints have explicit schemas and compatibility versions. Migration is tested, not treated as best-effort magic.

### Security tiers instead of one global performance trade-off

Trusted internal workloads may choose shared workers. Default cloud workloads receive process isolation. High-risk tenants or pages use container/microVM boundaries and stricter egress. Policy controls the choice.

### Reproducible agent programs with verification

A recorded workflow includes semantic locator intent, input schema, output schema, preconditions, action postconditions, retry budget, secret handles, approval requirements, and engine capability requirements. A workflow is successful only after verifying the intended state transition.

## Clean-room safeguards

1. Maintain a source provenance log for every dependency and generated artifact.
2. Keep public behavioral research separate from implementation notes.
3. Derive API behavior from standards and public protocol descriptions wherever possible.
4. Do not copy Lightpanda source, tests, comments, naming, or internal structures into the clean-room core.
5. Use independent designs and document them through ADRs.
6. Ask counsel to review the process before public release or migration of any third-party code.
7. Keep the product name, branding, UX, and public API distinct.

## Research questions that remain open until implementation evidence exists

- What percentage of the selected production corpus completes on the native fast path at each fidelity profile?
- Which Web APIs drive most fallback events?
- Can state migration preserve authenticated flows reliably across target sites?
- What is the optimal V8 isolate and worker topology under each isolation tier?
- Which semantic visibility features provide the best agent success per CPU cost?
- At equivalent success and isolation, where does Project Machina beat or lose to Lightpanda and Chromium?
- Does native Windows support justify its maintenance cost before GA?

## Official sources

- Lightpanda documentation index: https://lightpanda.io/docs/index
- Lightpanda Markdown and AXTree guide: https://lightpanda.io/docs/guides/markdown-axtree
- Lightpanda PandaScript documentation: https://lightpanda.io/docs/usage/pandascript
- Lightpanda agent tutorial: https://lightpanda.io/docs/guides/lightpanda-agent-tutorial
- Lightpanda repository: https://github.com/lightpanda-io/browser
- Lightpanda licensing statement: https://github.com/lightpanda-io/browser/blob/main/LICENSING.md

See [`SOURCES.md`](SOURCES.md) for the full research register.
