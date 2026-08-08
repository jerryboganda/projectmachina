---
title: "Product Roadmap"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Sequence delivery from governance and compatibility-first foundations through native runtime, agents, hardening, and GA."
---

# Product Roadmap

The milestone task files contain executable work. This roadmap communicates outcomes and dependency logic.

| Milestone | Outcome | Exit signal |
| --- | --- | --- |
| M0 — Foundation and governance | Monorepo, toolchains, agent loop, CI, contracts, threat/license baselines | Two agents can safely claim, build, review, and merge a small task |
| M1 — Compatibility-first platform | Chromium worker service, session API, command bus, control plane, initial protocols | A real task works end-to-end through one stable API with traces |
| M2 — Native engine fundamentals | Network, HTML, DOM, V8, event loop, navigation, extraction | Selected simple/dynamic pages complete natively |
| M3 — Native Web APIs and automation | Forms, storage, frames, workers, WebSockets, semantic actions | Initial agent/extraction corpus reaches useful native coverage |
| M4 — Protocols and SDKs | HTTP/gRPC maturity, CDP, BiDi, MCP, TypeScript/Python and other SDKs | Certified subset and version matrix available |
| M5 — Agent/workflow runtime | Recorder, DSL, replay, schemas, approvals, recovery | Successful interaction becomes deterministic repeatable workflow |
| M6 — Svelte console and DX | Developer/operator console, trace explorer, docs, CLI experience | Users can operate core journeys without raw API calls |
| M7 — Security and cloud operations | Isolation tiers, egress, auth, secrets, quotas, fleet, deployment | Controlled production beta environment is operable |
| M8 — Compatibility/performance hardening | WPT expansion, differential fixes, profiling, fuzzing, reliability | Release candidate meets target corpus and stability gates |
| M9 — Final certification and GA | Exhaustive final test campaign, audits, DR, benchmark, release | Authorized GA with reproducible evidence |

## Critical path

M0 contracts and tooling → M1 unified command bus/session model → M2 native lifecycle → M3 target APIs → M4 certified adapters → M5 deterministic workflows → M7 security/operations → M8 hardening → M9 certification.

M6 frontend can proceed in parallel after stable M1 API schemas. Quality harness work begins early but long suites run later.

## Planning ranges

Calendar duration depends on team experience and compatibility target. Use task throughput and critical-path evidence to forecast; do not treat early estimates as delivery promises. A team of 8–12 experienced engineers is the recommended production program, while a smaller team should explicitly reduce native capability scope and rely more heavily on fallback.
