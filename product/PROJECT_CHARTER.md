---
title: "Project Charter"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Authorize the project, define its mission, boundaries, governance, and completion standard."
---

# Project Charter

## Mission

Build a secure, adaptive, machine-native browser platform that completes automated web tasks with the lowest practical latency and resource cost while transparently escalating to full Chromium fidelity when the native engine cannot safely or correctly satisfy a capability.

## Product thesis

Conventional browsers optimize for pixels, media, human interaction, and broad backward compatibility. Machine workloads often need JavaScript, DOM, networking, forms, storage, semantic interaction, and deterministic extraction without paying the full rendering cost. Project Machina makes those needs first-class while retaining a compatibility path.

## Strategic objective

Win on **cost and latency per successfully verified task**, not on a narrow page-load benchmark. Performance, compatibility, reliability, and security are evaluated together.

## Authorized scope

- Native browser runtime optimized for automation and extraction.
- Embedded V8 JavaScript/WebAssembly execution.
- DOM, navigation, events, forms, networking, storage, frames, workers, and selected Web APIs.
- Semantic interaction kernel, markdown/structured extraction, and agent-native commands.
- Chromium fallback and state bridge.
- Unified typed command bus with HTTP, gRPC/events, CDP, WebDriver BiDi, MCP, and language SDKs.
- Deterministic recorded workflows and optional agent recovery.
- Local, self-hosted, and managed-cloud deployment models.
- SvelteKit operator/developer console.
- Security, isolation, observability, billing/metering foundations, and operations.

## Explicit non-goals for initial GA

- Replacing a consumer graphical browser.
- Native full-fidelity paint, font rasterization, GPU compositing, WebGL/WebGPU, or media playback.
- Browser extensions in the native engine.
- Perfect support for every Web API before launch.
- Circumventing anti-bot, access control, paywalls, rate limits, or site policies.
- Guaranteeing that every website runs natively; automatic fallback is an intentional feature.

## Sponsors and owners

| Accountability | Owner type |
| --- | --- |
| Product priorities and launch | Product/business owner |
| Architecture and technical quality | Principal architect |
| Security and privacy risk | Security owner |
| Licensing and IP | Legal owner |
| Production reliability and cost | Platform owner |
| Public performance claims | Independent technical reviewer |

## Governance

- Accepted ADRs govern architecture.
- Product changes update the PRD, scope, requirements, task graph, and acceptance criteria together.
- Security and legal gates cannot be waived by an implementation agent.
- Every public capability must map to executable evidence.
- Every benchmark claim must use fair, reproducible methodology.

## Completion standard

The charter is fulfilled only when M0–M9 are complete, the final heavy certification campaign passes, release and security approvals are recorded, artifacts are reproducible, rollback and disaster recovery are rehearsed, and the published capability matrix accurately matches observed behavior.
