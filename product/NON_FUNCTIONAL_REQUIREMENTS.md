---
title: "Non-Functional Requirements"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Set measurable targets for performance, reliability, security, usability, portability, and maintainability."
---

# Non-Functional Requirements

Targets are release gates unless marked observational. Final numbers may be revised only through evidence and product change control.

## Performance

| ID | Target |
| --- | --- |
| NFR-PERF-001 | Native warm session creation p95 ≤ 100 ms on reference hardware, excluding external network. |
| NFR-PERF-002 | Native command dispatch overhead p95 ≤ 5 ms for in-process operations, excluding page work. |
| NFR-PERF-003 | Hybrid mode demonstrates ≥95% successful completion on the selected corpus. |
| NFR-PERF-004 | Native fast path completes ≥80% of the initial target corpus or an approved evidence-based revision. |
| NFR-PERF-005 | Report CPU-seconds, memory-GB-seconds, network bytes, fallback cost, and successful task throughput. |
| NFR-PERF-006 | No unbounded queue, buffer, DOM growth, trace growth, or retry loop. |

## Reliability

| ID | Target |
| --- | --- |
| NFR-REL-001 | Controlled beta crash-free session rate ≥99.9%. |
| NFR-REL-002 | Every command supports a deadline or inherits a bounded session deadline. |
| NFR-REL-003 | Cancellation propagates to network, script, waits, and queued work. |
| NFR-REL-004 | Worker crashes do not corrupt durable tenant state or other isolated workers. |
| NFR-REL-005 | Idempotent create/mutation endpoints tolerate client retry. |
| NFR-REL-006 | Reproduction bundles are available for classified failures subject to retention/privacy policy. |

## Security and privacy

| ID | Target |
| --- | --- |
| NFR-SEC-001 | Treat all page content and script as hostile input. |
| NFR-SEC-002 | No production secret in source, ordinary log, trace, recording, or workflow definition. |
| NFR-SEC-003 | Tenant authorization is enforced server-side for every resource lookup and event stream. |
| NFR-SEC-004 | Local/private/metadata network access is denied by default in managed service. |
| NFR-SEC-005 | Critical/high vulnerabilities block release unless accepted by authorized owner with expiry. |
| NFR-SEC-006 | Reproducible SBOM, signed artifacts, dependency provenance, and vulnerability scanning. |

## Compatibility

| ID | Target |
| --- | --- |
| NFR-COMP-001 | Zero silent unsupported operations in certified protocol surfaces. |
| NFR-COMP-002 | Every supported client version has automated conformance evidence. |
| NFR-COMP-003 | Public API breaking changes follow version/deprecation policy. |
| NFR-COMP-004 | Native-vs-Chromium differential results are tracked by capability and runtime version. |

## Scalability

| ID | Target |
| --- | --- |
| NFR-SCALE-001 | Stateless control-plane instances scale horizontally. |
| NFR-SCALE-002 | Session scheduling applies tenant fairness and backpressure. |
| NFR-SCALE-003 | Worker pools can drain and recycle without accepting new sessions. |
| NFR-SCALE-004 | Durable data schema supports region partitioning without redesign. |

## Usability and accessibility

| ID | Target |
| --- | --- |
| NFR-UX-001 | Console meets WCAG 2.2 AA for implemented workflows, verified by automation and manual keyboard/screen-reader checks. |
| NFR-UX-002 | Critical session/fallback/error state is understandable without inspecting raw logs. |
| NFR-UX-003 | SDK quick start reaches a verified navigation/extraction result in under 15 minutes for a new developer. |

## Maintainability

| ID | Target |
| --- | --- |
| NFR-MAIN-001 | Component boundaries and public contracts are documented and enforced. |
| NFR-MAIN-002 | Unsafe Rust and C++/V8 boundary code has explicit invariants and focused tests. |
| NFR-MAIN-003 | Every feature flag has owner, default, telemetry, and removal condition. |
| NFR-MAIN-004 | Build and final certification are reproducible from pinned toolchains and lockfiles. |

## Portability

- Linux x86_64 and arm64 are P0.
- macOS x86_64/arm64 local developer binaries are P1.
- Native Windows x86_64 is a GA differentiation target; WSL is not the final supported experience.
- Container images use documented architecture and kernel requirements.
