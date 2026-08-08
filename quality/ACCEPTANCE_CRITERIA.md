---
title: "Program Acceptance Criteria"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define binary completion gates for beta, release candidate, final certification, and GA."
---

# Program Acceptance Criteria

## Documentation and traceability

- [ ] Requirements, architecture, protocols, security, operations, and task graph are current.
- [ ] Every required requirement maps to implementation and evidence.
- [ ] Capability matrix is generated from runtime/test source of truth.
- [ ] No known documentation claim contradicts observed behavior.

## Functional and compatibility

- [ ] Required P0 user journeys pass in local, self-hosted, and managed target modes as applicable.
- [ ] Hybrid verified-success target is met on selected corpus.
- [ ] Native fast-path target is met or formally revised before public commitment.
- [ ] Certified CDP/BiDi/MCP/SDK matrix passes.
- [ ] Unsupported behavior is explicit; silent unsupported count is zero in certified surfaces.
- [ ] Migration/replay protects side effects and reports omissions.

## Security and privacy

- [ ] Threat model and controls reflect final system.
- [ ] Tenant, sandbox, egress, secret-canary, prompt-injection, and approval tests pass.
- [ ] Critical/high findings closed or authorized according to release policy.
- [ ] SBOM/provenance/signatures and license review approved.
- [ ] Retention/deletion/artifact access verified.

## Performance and reliability

- [ ] Fair benchmark completed with raw evidence.
- [ ] Startup, memory, effective throughput, and tail-latency gates met.
- [ ] Crash-free and SLO readiness targets met.
- [ ] 24/72-hour soak and chaos invariants pass.
- [ ] Queue fairness, quotas, cancellation, recovery, and worker recycling pass.

## Delivery and operations

- [ ] Clean reproducible builds for supported targets.
- [ ] Installation, upgrade, rollback, canary, and worker drain pass.
- [ ] Backup/restore/DR meet approved RPO/RTO.
- [ ] On-call, incident, troubleshooting, and emergency controls are rehearsed.
- [ ] Monitoring, alerts, dashboards, and error budgets are operational.

## Frontend and developer experience

- [ ] Critical console journeys pass end-to-end.
- [ ] WCAG 2.2 AA target verified for implemented workflows.
- [ ] Performance/security budgets pass.
- [ ] SDK quick starts work from clean environments.
- [ ] Examples and docs links are validated.

## Approval

- [ ] Security owner signs release evidence.
- [ ] Legal/license owner approves distribution model.
- [ ] Independent reviewer approves public compatibility/performance claims.
- [ ] Product/release owner authorizes GA.
