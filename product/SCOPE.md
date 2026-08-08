---
title: "Product Scope and Change Control"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define in-scope and out-of-scope work and prevent uncontrolled browser-engine expansion."
---

# Product Scope and Change Control

## In scope through GA

### Native engine

- HTTP(S), redirects, DNS/TLS integration, cookies, cache policy.
- HTML parsing, DOM mutation, events, focus, forms, history, timers, microtasks.
- V8 JavaScript and WebAssembly integration.
- Fetch/XHR, WebSocket, workers, service workers where prioritized, storage APIs.
- Selectors, XPath, frames, Shadow DOM, custom elements.
- Semantic visibility/interactability and structured extraction.

### Compatibility engine

- Managed Chromium pool for screenshots, PDF, full CSS/layout, canvas, WebGL/WebGPU, media, extension-dependent, and unsupported API tasks.

### Platform

- Session routing, scheduling, quotas, isolation, state bridge, protocols, SDKs, workflows, observability, console, security, deployment, and operations.

## Out of scope through GA

- Native graphical renderer.
- Consumer browsing UI.
- Native media codecs and RTC stack.
- Native GPU APIs.
- Arbitrary browser extension execution in the native engine.
- Evasion systems designed to defeat security, anti-abuse, or access controls.
- Guaranteed compatibility with undocumented browser quirks not needed by target workloads.

## Scope admission test

A proposed feature enters scope only when it satisfies at least one:

- blocks a P0/P1 user journey;
- materially reduces fallback on the target corpus;
- closes a security/reliability gap;
- is required for a certified protocol/client claim;
- materially improves cost per successful task;
- is required by law, privacy, or contractual obligation.

It must also identify owner, acceptance evidence, dependencies, security impact, performance budget, and whether Chromium fallback is sufficient.

## Change-control procedure

1. Write the proposed change and user outcome.
2. Classify impact on product, architecture, protocol, security, data, schedule, and tests.
3. Update or add an ADR for architectural impact.
4. Update requirement IDs and capability matrix.
5. Add/remove/resequence task-graph nodes.
6. Obtain human approval only when the change hits a listed gate.
7. Preserve backward compatibility or publish a versioned migration.
