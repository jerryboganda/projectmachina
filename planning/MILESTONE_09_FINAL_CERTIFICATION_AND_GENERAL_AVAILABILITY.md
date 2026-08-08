---
title: "M9 — Final Certification and General Availability"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Program and Architecture"
purpose: "Provide executable task packets and exit criteria for milestone M9."
---

# M9 — Final Certification and General Availability

## Objective

Execute the consolidated heavy test campaign once on a frozen release candidate, repair only through controlled reruns, complete accountable approvals, and release the evidence-backed GA product.

## Entry criteria

- M8 independent RC readiness approved.
- Immutable candidate, test environments and evidence registry are ready.

## Exit criteria

- All program acceptance criteria and accountable approvals pass.
- GA artifacts, capability/compatibility/performance claims and operations match evidence.
- Autonomous task state is closed as complete with a distinct post-GA backlog.

## Scheduling notes

The orchestrator may run at most two implementation tasks concurrently. Parallel tasks must have non-overlapping write scopes and no unmerged shared-contract dependency. Every task follows `agents/AUTONOMOUS_LOOP.md` and records deferred heavy validation.

## Tasks

## M9-T01 — Freeze release candidate and initialize certification evidence registry

**Primary role:** release + quality  
**Dependencies:** M8-T12  
**Risk:** critical  
**May run in parallel:** no

### Deliverables

- Freeze source, toolchains, dependencies, engine/browser builds, schemas, corpora and environments.
- Create signed/hashed evidence registry with suite owners, commands, artifacts and failure IDs.
- Define controlled repair/impact/rerun procedure.

### Acceptance criteria

- Every campaign track references the same immutable candidate or documented repair candidate.
- Raw results and environment manifests are retained/classified.
- No test begins with unknown versions or success criteria.

### Fast gate

- Verify artifact signatures/hashes and environment manifests.
- Dry-run evidence upload/index validation.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M9-T02 — Run clean build, install, upgrade, migration and rollback certification

**Primary role:** release + platform  
**Dependencies:** M9-T01  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Build release artifacts on supported targets, compare reproducibility and verify SBOM/provenance/signatures.
- Install/package/start from clean environments and run quick starts.
- Upgrade supported prior state, execute data/workflow/schema migration, rollback and forward recovery.

### Acceptance criteria

- All release artifacts are traceable and installable.
- Upgrade preserves required data/behavior and rollback works within declared window.
- Any reproducibility variance is understood and approved.

### Fast gate

- Run full release build/install/upgrade/rollback track.
- Verify package/container signatures and notices.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M9-T03 — Run the full prioritized Web Platform Tests certification

**Primary role:** quality + native-engine  
**Dependencies:** M9-T01, M9-T02  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Execute all release-scope prioritized WPT shards on final candidate.
- Classify every failure/timeout/crash and compare to approved expectations/regressions.
- Repair release blockers and rerun impacted plus regression shards under controlled process.

### Acceptance criteria

- No unexplained regression or critical semantic/crash gap in enabled native capability.
- Expected limitations match capability matrix exactly.
- Result artifacts identify WPT revision, environment and engine build.

### Fast gate

- Run full WPT campaign and result consistency checks.
- Rerun affected shards after controlled repairs.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M9-T04 — Run the full native, hybrid and Chromium differential corpus

**Primary role:** quality + native-engine  
**Dependencies:** M9-T01, M9-T02  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Execute deterministic and approved real-task corpus across native-only, prefer-native/hybrid and Chromium reference.
- Compare lifecycle, DOM/semantic, network/storage, actions, workflow outputs and errors.
- Analyze native success, fallback, migration, retries and false-success.

### Acceptance criteria

- Hybrid and native targets are met or public claims are revised before release.
- No known false success, unsafe replay or undisclosed divergence in claimed workloads.
- External/site changes are distinguished with evidence.

### Fast gate

- Run full differential corpus with repeatability samples.
- Rerun repaired cases and related capability clusters.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M9-T05 — Run the full protocol, client and SDK conformance matrix

**Primary role:** protocol + quality  
**Dependencies:** M9-T01, M9-T02  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Execute HTTP/gRPC, certified Playwright/Puppeteer CDP, Selenium/BiDi, MCP and all SDK/runtime matrices.
- Test positive workflows plus auth, malformed input, unsupported, cancel, reconnect, slow consumer, backpressure and close.
- Generate final compatibility matrix.

### Acceptance criteria

- Every public certified tuple passes required suite.
- No silent unsupported or hang; errors map to canonical contract.
- Published versions/limitations equal evidence.

### Fast gate

- Run full conformance matrices.
- Run matrix-generation consistency check.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M9-T06 — Run extended fuzzing, sanitizers, penetration and security certification

**Primary role:** security + quality  
**Dependencies:** M9-T01, M9-T02  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Run multi-hour/multi-day priority fuzz and sanitizer targets on native/FFI/protocol/state/workflow paths.
- Run independent penetration/security review for auth, tenant, sandbox, egress, secrets, artifacts, prompt injection and approvals.
- Resolve findings and repeat canary secret scans/emergency controls.

### Acceptance criteria

- Zero unresolved critical/high release security finding under policy.
- No canary secret appears in any ordinary output/store/artifact.
- Isolation/egress/approval claims are directly evidenced.

### Fast gate

- Execute final fuzz/sanitizer/security campaign.
- Verify security report, fixes and targeted reruns.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M9-T07 — Run fair performance, load, capacity and cost certification

**Primary role:** performance + independent reviewer  
**Dependencies:** M9-T01, M9-T02, M9-T04  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Benchmark native/hybrid/Chromium and selected competitors on controlled equal-fidelity/equal-success workloads.
- Run concurrency/saturation/queue fairness/autoscaling and resource/cost measurement.
- Analyze startup, p50/p95/p99, CPU, memory, network, fallback, retries and cost per verified task.

### Acceptance criteria

- All reported results are reproducible with raw data/methodology.
- Release performance/resource gates pass without hidden success/fidelity differences.
- Independent reviewer approves any public comparison.

### Fast gate

- Run full benchmark/load campaign.
- Reproduce selected samples on independent host.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M9-T08 — Run 24-hour and 72-hour soak plus chaos certification

**Primary role:** quality + platform  
**Dependencies:** M9-T01, M9-T02, M9-T07  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Run 24-hour broad mixed native/Chromium/workflow load and 72-hour selected production-like workload.
- Inject worker/browser/control/network/storage/lease/deploy faults and observe recovery.
- Analyze memory/FD/thread/queue drift, crashes, fairness, artifacts and durable state.

### Acceptance criteria

- No critical/high leak, corruption, deadlock, cross-tenant effect or unsafe replay.
- Crash-free/SLO/resource targets pass or release is blocked/revised.
- Rollout, drain, circuit breaker and recovery work under sustained load.

### Fast gate

- Execute final soak and chaos plan.
- Verify invariant and resource trend reports.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M9-T09 — Certify Svelte console accessibility, security, performance and developer experience

**Primary role:** frontend + quality + security  
**Dependencies:** M9-T01, M9-T02  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Run all critical console route e2e, browser matrix, automated and manual accessibility including keyboard/screen reader.
- Run CSP/CSRF/XSS/auth/artifact isolation and route/bundle/load performance audits.
- Execute all SDK/docs quick starts and examples from clean environments.

### Acceptance criteria

- Implemented critical workflows meet approved WCAG 2.2 AA target.
- No critical/high frontend security issue or hostile-page content execution.
- Route/bundle performance and quick-start targets pass.

### Fast gate

- Run final frontend/DX campaign.
- Manual accessibility report and example logs attached.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M9-T10 — Certify backup, restore, disaster recovery, incident and emergency operations

**Primary role:** platform + security  
**Dependencies:** M9-T01, M9-T02, M9-T08  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Restore production-like backups/PITR and object artifacts into isolated environment.
- Execute disaster scenario, worker fleet recreation, credential rotation and measured RPO/RTO.
- Run incident command and emergency block/feature kill/rollback drills.

### Acceptance criteria

- Data integrity and approved RPO/RTO are met.
- Side-effecting workflows recover only from verified safe checkpoint.
- Runbooks/alerts/roles work and drill actions are audited.

### Fast gate

- Run final restore/DR/incident technical exercise.
- Verify retention/deletion and emergency control outcomes.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M9-T11 — Complete legal, license, privacy, release documentation and public-claim review

**Primary role:** legal + release + product  
**Dependencies:** M9-T02 through M9-T10  
**Risk:** critical  
**May run in parallel:** no

### Deliverables

- Review license choice, clean-room record, SBOM/notices, V8/Chromium/test corpus redistribution and branding.
- Approve privacy/retention/region/subprocessor/acceptable-use documentation.
- Finalize release notes, capability/client matrix, known limitations, benchmark and security summaries.

### Acceptance criteria

- No unresolved distribution/license/branding gate.
- Every public claim maps to independently reviewed evidence and names limitations.
- Customer/operator docs match final build and policy.

### Fast gate

- Run release-document/evidence consistency audit.
- Verify notices, package metadata and public links.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M9-T12 — Obtain final approvals, release GA and close the autonomous program loop

**Primary role:** release + product owner  
**Dependencies:** M9-T01 through M9-T11  
**Risk:** critical  
**May run in parallel:** no

### Deliverables

- Assemble final signed/hashed certification report, residual-risk register and release checklist.
- Obtain security, legal, platform, independent claim and product/release approvals.
- Promote immutable artifacts through canary to GA, monitor gates and record rollback readiness.

### Acceptance criteria

- All program acceptance criteria pass or authorized residual risks are explicit with expiry.
- GA uses the certified artifacts/config and monitoring confirms healthy launch.
- Task graph/state closes as COMPLETE and post-GA backlog is separated from required scope.

### Fast gate

- Run final acceptance checklist and launch canary smoke.
- Verify state/evidence/archive and rollback command.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.
