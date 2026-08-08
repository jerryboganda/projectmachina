---
title: "On-Call Operations"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define support ownership, severity, handoff, escalation, and safe automation for production operations."
---

# On-Call Operations

## Coverage roles

- Primary operations responder.
- Secondary/runtime specialist.
- Security responder on security indicators.
- Incident commander for SEV-0/1.
- Communications/product/legal owners as required.

## Severity

| Severity | Example | Response |
| --- | --- | --- |
| SEV-0 | widespread compromise, cross-tenant exposure, global outage/data loss | immediate incident command and containment |
| SEV-1 | major region outage, severe verified-success drop, active high-risk exploit | urgent response and promotion halt |
| SEV-2 | partial degradation, one worker class failure, elevated fallback/crash | timely mitigation and tracked incident |
| SEV-3 | isolated defect, low-impact alarm, non-urgent operational task | normal queue |

## Handoff

Include active incidents, release/config changes, error-budget burn, worker pool anomalies, security alerts, pending migrations, capacity/cost risks, and exact next checks. Use durable incident records, not chat memory.

## Safe automation

Automations may page, stop rollout, drain known-bad worker versions, scale within approved bounds, or trip a circuit breaker according to runbook. They may not accept security risk, delete evidence/data, exceed spending limits, or make public communication.

## Alert quality

Every page has user/security impact, threshold, dashboard, runbook, owner, and deduplication. Remove or tune unactionable alerts. Synthetic success probes supplement component health.

## Post-incident

SEV-0/1 and recurring SEV-2 events require review and tracked actions. Feed production reproductions into tests and capability/routing decisions without retaining unnecessary sensitive content.
