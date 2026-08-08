---
title: "M7 — Security and Cloud Operations"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Program and Architecture"
purpose: "Provide executable task packets and exit criteria for milestone M7."
---

# M7 — Security and Cloud Operations

## Objective

Harden execution and tenancy, deploy the managed architecture, operationalize egress/secrets/quotas/fleet/backup/SLOs, and run a tightly controlled production beta.

## Entry criteria

- Core engine/protocol/workflow journeys are functional.
- Security and cloud owner access is available for approved environments.

## Exit criteria

- Dedicated and hardened isolation, egress, auth, secrets and abuse controls are evidenced.
- Production topology, fleet, backups, SLOs and incident controls are operational.
- Controlled beta provides real evidence for hardening without uncontrolled risk.

## Scheduling notes

The orchestrator may run at most two implementation tasks concurrently. Parallel tasks must have non-overlapping write scopes and no unmerged shared-contract dependency. Every task follows `agents/AUTONOMOUS_LOOP.md` and records deferred heavy validation.

## Tasks

## M7-T01 — Harden dedicated-process worker sandbox

**Primary role:** security + platform  
**Dependencies:** M1-T05, M3-T15, M0-T06  
**Risk:** critical  
**May run in parallel:** constrained

### Deliverables

- Run each untrusted session or approved group in an unprivileged process/container boundary with namespaces/cgroups/seccomp/LSM policy.
- Enforce read-only root, isolated ephemeral storage, PID/file/socket limits and scoped execution grant.
- Document kernel/platform requirements and failure diagnostics.

### Acceptance criteria

- Worker cannot access host secrets, sockets, filesystem or unauthorized processes.
- CPU/memory/PID/disk limits terminate/classify abuse without host instability.
- Native and Chromium dedicated workers pass the same baseline isolation contract.

### Fast gate

- Run sandbox negative and resource exhaustion suite.
- Inspect effective runtime policy in CI Linux.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M7-T02 — Validate and constrain shared-performance isolation tier

**Primary role:** security + native-engine  
**Dependencies:** M7-T01, M2-T14  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Define eligible tenants/workloads and explicit risk disclosure for shared process mode.
- Implement complete session reset/reuse invariants and identity-safe pools.
- Create cross-session state, crash, timing/resource and connection-pool isolation tests.

### Acceptance criteria

- Seeded DOM/cookie/storage/secret/proxy/telemetry data never appears in next session.
- Shared crash blast radius and recovery match documented policy.
- Untrusted workloads default to dedicated tier unless approved.

### Fast gate

- Run repeated cross-session canary suite.
- Run crash/resource/noisy-neighbor tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M7-T03 — Deliver hardened container or microVM isolation tier

**Primary role:** security + platform  
**Dependencies:** M7-T01  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Select and implement hardened container/microVM runtime with dedicated network/filesystem/kernel boundary as approved.
- Integrate scheduler provisioning, attestation/health, warm/cold policy and teardown.
- Apply stronger artifact, profile and secret restrictions.

### Acceptance criteria

- Hardened session cannot access other workload/host namespace in negative tests.
- Provision/cancel/teardown is bounded and observable.
- Policy never silently falls back to weaker tier when capacity is unavailable.

### Fast gate

- Run isolation and lifecycle suite on target infrastructure.
- Run capacity unavailable/policy downgrade tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M7-T04 — Enforce production network egress, SSRF and DNS rebinding controls

**Primary role:** security + platform  
**Dependencies:** M3-T06, M3-T10, M7-T01  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Implement URL/scheme/domain/port/DNS/IP/redirect/proxy policy at worker network boundary.
- Deny private/link-local/metadata/reserved ranges by default and revalidate connection destination.
- Add per-origin limits, WebSocket/download controls and emergency blocks.

### Acceptance criteria

- Encoded/IPv4/IPv6/rebinding/redirect/proxy fixtures cannot reach denied targets.
- Allowed public fixture works through native and Chromium.
- Policy events contain safe reason without leaking credentials/content.

### Fast gate

- Run full SSRF/rebinding fixture suite now.
- Run emergency block propagation smoke.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M7-T05 — Harden authentication, authorization and just-in-time administration

**Primary role:** security + platform  
**Dependencies:** M1-T02, M6-T02  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Integrate production identity provider/service credentials and scoped roles.
- Implement JIT/time-bounded privileged elevation and stronger admin authentication.
- Expand authorization tests for every resource/event/artifact/admin operation.

### Acceptance criteria

- Cross-tenant and privilege-escalation suite passes.
- Revoked/expired credentials and grants stop new access promptly.
- Privileged actions require reason and audit.

### Fast gate

- Run complete authorization matrix.
- Run token replay/revocation/session fixation tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M7-T06 — Integrate production secret vault and centralized redaction

**Primary role:** security + platform  
**Dependencies:** M5-T07, M7-T01  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Connect approved vault/KMS abstraction with scoped short-lived worker grants.
- Centralize log/trace/recording/artifact/console redaction and canary scanning.
- Implement rotation, revocation and secret-use audit.

### Acceptance criteria

- Canary secret traverses representative native/Chromium/workflow paths without artifact exposure.
- Worker cannot list or retrieve unrelated secrets.
- Rotation/revocation affects future use and fails safely.

### Fast gate

- Run end-to-end canary and authorization suite.
- Scan stored artifacts and frontend state.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M7-T07 — Implement quotas, rate limits, abuse controls and responsible crawling defaults

**Primary role:** security + platform  
**Dependencies:** M1-T02, M7-T04  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Enforce organization/project/key/session/origin quotas for concurrency, CPU/memory, requests/bytes, fallback and artifacts.
- Add robots/polite crawler profile, anomaly signals and tenant/destination emergency blocks.
- Expose safe usage and quota events/APIs.

### Acceptance criteria

- Limit exceedance is typed, fair and cannot create unbounded queue.
- Crawler defaults honor configured policy and identify behavior where required.
- Suspended key/project/tenant stops new work without affecting others.

### Fast gate

- Run quota/fairness/rate fixtures.
- Run abuse/emergency suspension smoke.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M7-T08 — Create Kubernetes, Helm and infrastructure-as-code production topology

**Primary role:** platform + security  
**Dependencies:** M7-T01, M7-T03, M1-T12  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Package control plane, console and worker pools with probes, resources, security contexts, network policy and disruption controls.
- Create versioned Helm/IaC for one launch region and environment separation.
- Use workload identity, managed data services and signed images.

### Acceptance criteria

- A clean staging environment deploys from code and passes health/smoke.
- Policy prevents privileged/root/unsigned/unapproved workloads.
- Configuration and migrations are versioned and rollback capable.

### Fast gate

- Run IaC/manifest policy and deployment smoke.
- Run rolling upgrade/drain/rollback in staging.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M7-T09 — Implement autoscaling, fleet operations, metering and cost controls

**Primary role:** platform  
**Dependencies:** M1-T04, M7-T08, M6-T08  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Scale separate pools by engine/isolation/resource class with warm targets and caps.
- Implement drain/recycle/circuit-breaker/version rollout and admin APIs.
- Emit reconcilable usage events and budget/fallback-cost controls.

### Acceptance criteria

- Synthetic load scales correct pool without violating tenant fairness or budget cap.
- Bad worker version can be canaried, drained and rolled back.
- Usage aggregates reconcile to session/worker events.

### Fast gate

- Run scale/drain/canary/cost simulation.
- Run duplicate/missing usage event reconciliation tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M7-T10 — Implement production backup, restore, retention and disaster-recovery foundations

**Primary role:** platform + security  
**Dependencies:** M1-T01, M7-T08  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Configure encrypted database backup/PITR, object retention/versioning and restore automation.
- Implement retention/deletion jobs and legal-hold controls.
- Create isolated restore verification and DR bootstrap scripts.

### Acceptance criteria

- A staging backup restores to isolated environment with integrity checks.
- Synthetic deletion removes authorized data across stores/projections.
- Workers/control plane can be recreated from signed artifacts.

### Fast gate

- Run restore and deletion smoke.
- Run backup access/authorization checks.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M7-T11 — Operationalize SLOs, alerting, on-call, incident and emergency controls

**Primary role:** platform + security  
**Dependencies:** M7-T08, M7-T09, M7-T10  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Implement verified-success SLI dashboards, burn alerts, queue/crash/fallback/security alerts and runbook links.
- Create on-call/incident roles, communication and post-incident workflow.
- Rehearse feature kill, worker drain, credential/tenant/domain block and rollback.

### Acceptance criteria

- Alerts are actionable and synthetic failure pages expected owners.
- Emergency actions are authorized, audited and reversible where possible.
- Incident drill produces complete timeline and remediation list.

### Fast gate

- Run alert injection and tabletop/technical drill.
- Verify no sensitive data in incident artifacts.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M7-T12 — Launch and evaluate a controlled production beta

**Primary role:** product + release + security  
**Dependencies:** M7-T01 through M7-T11, M4-T11, M5-T10, M6-T10  
**Risk:** critical  
**May run in parallel:** no

### Deliverables

- Complete security/legal/cloud approval gates and define beta tenants/workloads/limits.
- Deploy immutable canary, onboard controlled users and collect verified success/fallback/reliability/cost feedback.
- Produce beta exit/defect/prioritization report without unsupported public claims.

### Acceptance criteria

- Beta operates within SLO/security/cost guardrails and incident response is available.
- Critical/high findings are contained and tracked.
- Telemetry identifies native gaps and M8 hardening priorities.

### Fast gate

- Run beta readiness suite and post-launch synthetic monitoring.
- Review real workload evidence under privacy policy.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.
