---
title: "Service Level Objectives and Error Budgets"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define initial reliability objectives, measurement boundaries, alerts, and release policies."
---

# Service Level Objectives

Final commercial commitments require owner approval and beta evidence. These are engineering objectives.

## Proposed SLIs/SLOs

| Service journey | SLI | Initial objective |
| --- | --- | --- |
| Session creation | valid sessions reaching ready within class deadline | 99.9% monthly excluding customer quota/policy errors |
| Command execution | eligible commands reaching verified terminal outcome | 99.9% monthly excluding declared page/policy failures |
| Event stream | ordered events delivered/resumable within retention | 99.9% |
| Workflow run | platform-eligible runs completing or classified | 99.9% |
| Control API | non-browser control requests successful | 99.95% |
| Artifact metadata/download | authorized available artifacts accessible | 99.9% |

Latency objectives are defined by workload/engine class rather than one global number.

## Measurement

Measure at server side using canonical outcomes. Exclude only documented customer-caused invalid/policy/quota errors and approved maintenance; page/browser failures still count in task success metrics when the product claims to handle them.

## Error budgets

Budget consumption governs release velocity. Fast burn triggers incident and promotion halt. Sustained exhaustion prioritizes reliability work over new features. Security or cross-tenant failures are not traded against an availability budget.

## Alerts

Multi-window burn-rate alerts, verified-success drop, crash/fallback anomaly, queue saturation, event gaps, artifact/storage errors, and security control failures. Alerts include runbook and correlation dashboard.

## Reporting

By region, engine policy, isolation class, release, and major workload without exposing tenant-sensitive content. Public SLO commitments name exclusions and measurement method.
