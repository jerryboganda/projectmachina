---
title: "Milestone Dependency Map"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Explain critical path, parallel lanes, shared contracts, and milestone-level dependencies."
---

# Dependency Map

## Milestone graph

```text
M0 Foundation
  -> M1 Compatibility-first platform
      -> M2 Native fundamentals
          -> M3 Native automation/Web APIs
              -> M4 Protocol certification
                  -> M5 Deterministic workflows
      -> M6 Svelte console (begins after stable M1 API/contracts)
      -> M7 Security/cloud operations (foundations begin earlier; beta gate here)
M3 + M4 + M5 + M6 + M7
  -> M8 Hardening
      -> M9 Final certification and GA
```

## Critical shared contracts

Serialize changes to:

- canonical command/event/error model;
- capability registry/schema;
- session/policy/domain schema;
- protobuf/OpenAPI/workflow schemas;
- state-transfer format;
- root build/lockfiles and release configuration;
- authentication/authorization model.

Dependent implementations begin after the relevant contract merges.

## Recommended parallel lanes

| Stage | Lane A | Lane B |
| --- | --- | --- |
| M0 | monorepo/CI/agent tooling | contracts/security/test harness |
| M1 | session/scheduler/Chromium | APIs/events/artifacts/SDK |
| M2 | HTML/DOM/V8/event loop | network/storage/extraction/worker |
| M3 | forms/frames/workers | semantic/interception/state bridge |
| M4 | CDP/BiDi | HTTP/gRPC/MCP/SDKs |
| M5–M6 | workflow runtime | Svelte console/docs |
| M7 | sandbox/egress/auth | cloud/fleet/backup/SLO |
| M8 | standards/differential/fuzz | performance/platform/cross-platform |
| M9 | standards/protocol/security | performance/soak/frontend/DR |

## Cross-milestone quality work

Harnesses begin in M0. Focused checks accompany every task. Broad execution is scheduled M8/M9, so test infrastructure must not be postponed until M9.
