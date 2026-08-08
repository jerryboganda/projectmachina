---
title: "Product Requirements Document"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Describe users, problems, product behavior, priorities, and release outcomes for the complete platform."
---

# Product Requirements Document

## Executive summary

Project Machina is a browser runtime and automation platform for AI agents, extraction systems, test automation, and high-concurrency web tasks. It provides a fast native execution path and a full-browser compatibility path behind one stable API. Users choose policy and fidelity; the system chooses the lightest engine that can complete the task correctly.

## Problem

Teams running browser automation commonly face:

- high memory and CPU cost per session;
- slow startup and low density;
- expensive protocol round trips and serialization;
- fragile selector-driven agents;
- unsupported behavior that appears as ambiguous timeouts;
- difficult state migration between lightweight and full browsers;
- poor debugging of nondeterministic workflows;
- security risk from executing hostile web content;
- fragmented interfaces across Playwright, Puppeteer, Selenium, MCP, and proprietary APIs.

## Product principles

1. **Correct outcome before benchmark speed.** A fast wrong task is a failure.
2. **Adaptive fidelity.** Pay only for capabilities the task needs.
3. **Fallback is explicit.** Users always know which engine ran and why.
4. **One behavior model.** All protocols call the same typed commands.
5. **Machine-native outputs.** Semantic trees, structured data, and deltas are first-class.
6. **Determinism after discovery.** Successful agent workflows become replayable programs.
7. **Security by policy tier.** Isolation, egress, secrets, and approvals are built in.
8. **Evidence over claims.** Capability and performance statements are executable.

## Primary personas

- AI agent platform engineer.
- Data extraction/crawling engineer.
- Browser automation/test engineer.
- Platform/SRE administrator.
- Security/compliance administrator.
- Workflow author and operations analyst.

See `PERSONAS_AND_JOBS.md` for detailed jobs and outcomes.

## Product surfaces

### Runtime

- Local binary and embeddable/native API.
- Self-hosted worker service.
- Managed browser-session service.

### APIs and protocols

- HTTP/JSON control API.
- gRPC command/event stream.
- Chrome DevTools Protocol compatibility surface.
- WebDriver BiDi compatibility surface.
- MCP server and agent-native tools.
- TypeScript, Python, Go, Java, and Rust SDKs.

### Console

- Organizations, projects, API keys, policies, and usage.
- Session launch, inspection, traces, recordings, and replay.
- Workflow authoring, versions, run history, and approvals.
- Capability/fallback reports and performance diagnostics.
- Worker fleet, incidents, quotas, and billing views where applicable.

## Core user journey

1. A developer creates a project and policy.
2. The developer launches a session with a fidelity profile.
3. The capability router starts the task in the native engine when eligible.
4. Commands run through the unified command model.
5. Unsupported requirements either produce an explicit typed error or migrate to Chromium according to policy.
6. The platform returns output plus engine, fallback, resource, trace, and verification metadata.
7. A successful interactive task can be recorded into a deterministic workflow.
8. Repeated runs execute the workflow without an LLM unless recovery is configured.

## Functional epics

### E1 — Session and context management

Create, configure, cancel, inspect, persist, and close isolated browser sessions and contexts. Support deadlines, quotas, profiles, proxies, locale, timezone, viewport, permissions, cookies, and storage.

### E2 — Native engine

Implement standards-oriented navigation, parsing, DOM, JavaScript, events, forms, fetch/XHR, storage, selectors, frames, workers, WebSockets, and semantic interaction needed by priority workloads.

### E3 — Capability routing and fallback

Predict capability needs, observe runtime unsupported APIs, apply resource/fidelity policy, migrate state, replay actions when required, and report every fallback decision.

### E4 — Protocol compatibility

Expose the same internal behavior through native SDKs, HTTP, gRPC/events, CDP, BiDi, and MCP. Pin and publish supported client versions and command modules.

### E5 — Agent and deterministic workflows

Expose semantic roles/names/states, stable locators, action preconditions/postconditions, schema extraction, recording, compilation, versioning, replay, recovery, secrets, and approvals.

### E6 — Security and tenancy

Provide authentication, authorization, tenant boundaries, sandbox tiers, network controls, secret redaction, audit logs, quotas, abuse controls, and incident readiness.

### E7 — Developer and operator experience

Provide SvelteKit console, CLI, SDK examples, traces, reproduction bundles, docs, local development, deployments, and health/usage views.

### E8 — Reliability and performance

Provide structured errors, cancellation, crash containment, worker recycling, observability, benchmarking, WPT/differential/fuzz/conformance testing, load and soak evidence.

## Release priorities

| Priority | Meaning |
| --- | --- |
| P0 | Required for first credible production beta |
| P1 | Required for GA or immediate post-beta certification |
| P2 | Valuable expansion after stable GA |

### P0

- Compatibility-first Chromium service.
- Native fast path for the chosen extraction/agent corpus.
- Transparent fallback.
- HTTP/gRPC, MCP, basic CDP/BiDi, TypeScript/Python SDKs.
- Deterministic workflows.
- Three isolation policies, at least shared and dedicated operational at beta.
- Traces, reproduction bundles, quotas, audit, and SvelteKit console.

### P1

- Broader frames/workers/storage/interception compatibility.
- Certified Playwright/Puppeteer/Selenium client matrix.
- Hardened microVM tier.
- Go/Java/Rust SDK maturity.
- Advanced workflow repair and selector healing.
- Multi-region worker placement with region policy.

### P2

- Additional native APIs based on fallback telemetry.
- Domain optimization profiles.
- Marketplace/integration ecosystem.
- Browser extension compatibility through fallback.
- Advanced visual validation and media workloads through specialized workers.

## Product constraints

- Native behavior must not claim visual fidelity it does not implement.
- Chromium state migration may be replay-based where exact in-memory transfer is impossible; the distinction is disclosed.
- Public APIs are versioned and compatibility-tested.
- Hostile page code is assumed.
- Cost controls apply per tenant, session, origin, and workflow.
- Legal and responsible-use policies apply to crawling and automation.

## Beta exit criteria

- At least 95% successfully verified completion in hybrid mode on the selected production corpus.
- At least 80% native completion on the initial agent/extraction corpus, or a documented revised gate supported by data.
- No silent unsupported command behavior in certified surfaces.
- Crash-free session target of 99.9% in controlled beta load.
- All critical/high security findings closed or explicitly accepted by authorized owner.
- Reproducible build, deployment, rollback, and incident runbook.

## GA exit criteria

- M9 final certification complete.
- Certified client/protocol matrix published.
- SLOs and error budgets operational.
- Disaster recovery and restore rehearsed.
- License/SBOM review approved.
- Performance claims independently reviewed.
- Data retention, privacy, and regional controls approved.
