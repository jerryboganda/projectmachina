---
title: "Functional Requirements"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Provide traceable, testable functional requirements for the runtime, protocols, workflows, console, and platform."
---

# Functional Requirements

Each requirement has a stable ID. Implementation tasks, tests, and release evidence must reference these IDs.

## Session and policy

| ID | Requirement | Priority |
| --- | --- | --- |
| FR-SES-001 | Create, retrieve, cancel, and close sessions idempotently. | P0 |
| FR-SES-002 | Configure fidelity profile, fallback policy, isolation tier, deadline, request/byte/memory budgets, locale, timezone, viewport, headers, proxy, and permissions. | P0 |
| FR-SES-003 | Support multiple isolated browser contexts within policy limits. | P1 |
| FR-SES-004 | Emit ordered lifecycle and command events with stable correlation IDs. | P0 |
| FR-SES-005 | Return engine, capability, fallback, resource, and verification metadata. | P0 |

## Native browser

| ID | Requirement | Priority |
| --- | --- | --- |
| FR-NAT-001 | Navigate HTTP(S) URLs with redirects, cancellation, deadlines, and lifecycle states. | P0 |
| FR-NAT-002 | Parse conforming HTML into a mutable DOM and expose selectors/XPath. | P0 |
| FR-NAT-003 | Execute JavaScript and WebAssembly through isolated V8 contexts. | P0 |
| FR-NAT-004 | Implement event loop, tasks, microtasks, timers, events, focus, forms, and history needed by target workloads. | P0 |
| FR-NAT-005 | Implement fetch/XHR, cookies, local/session storage, and prioritized storage/network APIs. | P0 |
| FR-NAT-006 | Support Shadow DOM, custom elements, frames, workers, and WebSockets according to capability matrix. | P1 |
| FR-NAT-007 | Produce semantic tree, interactive index, markdown, links, headings, forms, and schema extraction from one live DOM. | P0 |
| FR-NAT-008 | Expose task-ready wait conditions and semantic-stability revisions. | P0 |

## Routing and fallback

| ID | Requirement | Priority |
| --- | --- | --- |
| FR-RTE-001 | Select engine using declared policy, predicted needs, and observed capabilities. | P0 |
| FR-RTE-002 | Never silently ignore an unsupported command or Web API. | P0 |
| FR-RTE-003 | Support `native-only`, `prefer-native`, `prefer-compatible`, and `chromium-only` modes. | P0 |
| FR-RTE-004 | Transfer transferable state and replay verified actions when migration requires it. | P0 |
| FR-RTE-005 | Report reason, timing, state method, and cost impact of fallback. | P0 |
| FR-RTE-006 | Maintain domain/API fallback telemetry without storing sensitive page content by default. | P0 |

## Protocols and SDKs

| ID | Requirement | Priority |
| --- | --- | --- |
| FR-API-001 | All adapters invoke one versioned typed command model. | P0 |
| FR-API-002 | Provide HTTP/JSON and gRPC/event-stream APIs with idempotency, cancellation, deadlines, and pagination. | P0 |
| FR-API-003 | Provide documented CDP and WebDriver BiDi compatibility subsets. | P0 |
| FR-API-004 | Provide MCP tools/resources/prompts for session, navigation, interaction, extraction, trace, and workflow operations. | P0 |
| FR-API-005 | Provide TypeScript and Python SDKs at beta; Go, Java, and Rust by GA. | P0/P1 |
| FR-API-006 | Publish protocol/client version compatibility and deprecation policy. | P0 |

## Workflows and agents

| ID | Requirement | Priority |
| --- | --- | --- |
| FR-WF-001 | Record a session into a typed deterministic workflow with variables and secret references. | P0 |
| FR-WF-002 | Verify action preconditions and postconditions and record DOM/semantic revisions. | P0 |
| FR-WF-003 | Version, test, approve, schedule, run, pause, resume, and roll back workflows. | P0 |
| FR-WF-004 | Execute normal workflow runs without an LLM. | P0 |
| FR-WF-005 | Offer optional bounded recovery and selector repair with explicit cost and approval policy. | P1 |
| FR-WF-006 | Require approval policy for configured high-impact actions. | P0 |

## Security and tenancy

| ID | Requirement | Priority |
| --- | --- | --- |
| FR-SEC-001 | Authenticate users/services and authorize organization/project/session operations. | P0 |
| FR-SEC-002 | Enforce shared, dedicated-process, and hardened isolation policies as offered. | P0/P1 |
| FR-SEC-003 | Enforce network egress, DNS rebinding, local-address, metadata-service, scheme, and download policies. | P0 |
| FR-SEC-004 | Resolve secrets through opaque references and redact logs/traces/recordings. | P0 |
| FR-SEC-005 | Produce tenant-scoped audit records for security- and policy-relevant actions. | P0 |
| FR-SEC-006 | Enforce quotas, rate limits, responsible-use controls, and emergency blocking. | P0 |

## Console and operations

| ID | Requirement | Priority |
| --- | --- | --- |
| FR-UI-001 | Provide responsive, keyboard-accessible SvelteKit console for projects, policies, sessions, workflows, traces, usage, and administration. | P0 |
| FR-UI-002 | Stream session lifecycle and trace updates without requiring full-page refresh. | P0 |
| FR-UI-003 | Display explicit engine/fallback/capability/error metadata. | P0 |
| FR-OPS-001 | Expose health, readiness, metrics, logs, traces, profiles, and redacted reproduction bundles. | P0 |
| FR-OPS-002 | Support fleet draining, worker recycling, rollout, rollback, backup, and restore. | P0 |
| FR-OPS-003 | Meter resource usage and enforce project/tenant budgets. | P0 |
