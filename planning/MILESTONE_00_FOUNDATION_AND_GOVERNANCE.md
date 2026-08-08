---
title: "M0 — Foundation and Governance"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Program and Architecture"
purpose: "Provide executable task packets and exit criteria for milestone M0."
---

# M0 — Foundation and Governance

## Objective

Create the repository, contracts, security/quality baselines, deterministic development environment, and safe multi-agent operating system required by every later milestone.

## Entry criteria

- Documentation baseline accepted or recommended defaults in force.
- A Git repository and at least one supported development environment are available.

## Exit criteria

- Two coding agents can safely execute non-overlapping tasks through reviewed merge.
- Minimal polyglot workspace builds and CI fast gate is reliable.
- Canonical schemas, threat/license baselines, fixtures, telemetry and benchmark harness exist.

## Scheduling notes

The orchestrator may run at most two implementation tasks concurrently. Parallel tasks must have non-overlapping write scopes and no unmerged shared-contract dependency. Every task follows `agents/AUTONOMOUS_LOOP.md` and records deferred heavy validation.

## Tasks

## M0-T01 — Initialize the polyglot monorepo and pinned toolchains

**Primary role:** platform  
**Dependencies:** none  
**Risk:** high  
**May run in parallel:** constrained

### Deliverables

- Create the repository structure from `architecture/REPOSITORY_STRUCTURE.md`.
- Pin Rust, Node/pnpm, Clang/CMake, protobuf/Buf, V8 and Chromium acquisition metadata.
- Add `just` commands for doctor, build, format, check, test selection and clean setup.

### Acceptance criteria

- A clean supported host can bootstrap without undocumented manual steps.
- Minimal Rust, C++ bridge stub and SvelteKit workspace compile.
- Lockfiles and toolchain versions are committed and machine-readable.

### Fast gate

- Run bootstrap/doctor in a clean container or VM.
- Run format and minimal cross-language build.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M0-T02 — Implement task claims, leases, worktree and evidence helpers

**Primary role:** platform  
**Dependencies:** M0-T01  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Create scripts/CLI to claim task and path globs atomically, renew heartbeat, release and inspect claims.
- Create and clean task worktrees/branches safely.
- Write machine-readable task/evidence state and human-readable projections.

### Acceptance criteria

- Overlapping write claims are rejected deterministically.
- Expired claims can be recovered only through documented inspection.
- Two agents can create separate worktrees and state survives session restart.

### Fast gate

- Unit-test claim overlap, lease expiry and path normalization.
- Run a two-worktree smoke with concurrent non-overlapping claims.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M0-T03 — Configure protected fast-gate CI and repository policy

**Primary role:** platform + security  
**Dependencies:** M0-T01, M0-T02  
**Risk:** high  
**May run in parallel:** constrained

### Deliverables

- Add pull-request workflows for changed-area build/type/test, contracts, docs, secrets and dependency checks.
- Add CODEOWNERS, PR template, labels and merge-queue policy documentation.
- Enforce task/path ownership and generated-file cleanliness.

### Acceptance criteria

- A valid small PR completes within the fast-gate target on warm CI.
- A secret, dirty generated output or overlapping claim fails the gate.
- Untrusted PR jobs have no release/production credentials.

### Fast gate

- Run positive and deliberately failing CI fixtures.
- Inspect workflow permissions and pinned third-party actions.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M0-T04 — Create the canonical command, event, error and capability schema skeleton

**Primary role:** architect + protocol  
**Dependencies:** M0-T01  
**Risk:** critical  
**May run in parallel:** constrained

### Deliverables

- Define versioned schema/types for command envelope, outcome, event, canonical error and capability status.
- Generate initial Rust and TypeScript bindings plus round-trip fixtures.
- Document extension and compatibility rules.

### Acceptance criteria

- Schemas represent session create, navigation, semantic query, click and close without transport types.
- Unknown/additive fields follow the declared policy.
- Generated outputs are deterministic and source-hashed.

### Fast gate

- Run schema round-trip and compatibility checks.
- Compile generated Rust/TypeScript consumers.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M0-T05 — Enforce architecture boundaries and ADR workflow

**Primary role:** architect  
**Dependencies:** M0-T01, M0-T04  
**Risk:** medium  
**May run in parallel:** yes

### Deliverables

- Add dependency-boundary checks/lints for crates, services, protocols and frontend.
- Create ADR tooling/template and accepted-ADR validation.
- Add a generated architecture dependency report.

### Acceptance criteria

- Protocol adapters cannot import engine internals.
- Frontend consumes generated contracts rather than database or hand-written duplicates.
- Boundary violation fixture fails CI with an actionable message.

### Fast gate

- Run dependency graph/boundary check.
- Test a deliberate forbidden import in a fixture.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M0-T06 — Establish the executable security baseline and threat controls

**Primary role:** security  
**Dependencies:** M0-T01, M0-T04  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Translate the threat model into security requirements/test tags.
- Add baseline secure configuration, secret/redaction policy and development permission model.
- Create security issue and approval workflows.

### Acceptance criteria

- Critical trust boundaries have owners and planned tests.
- Development/CI cannot access production secrets by default.
- A seeded canary secret is detected in a test artifact.

### Fast gate

- Run configuration and canary redaction smoke.
- Review threat-to-task traceability.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M0-T07 — Create SBOM, provenance, license and clean-room controls

**Primary role:** security + release  
**Dependencies:** M0-T01  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Generate dependency/SBOM inventories for Rust, C++, Node and containers.
- Add license policy and third-party provenance record.
- Add clean-room contribution checklist and release notices skeleton.

### Acceptance criteria

- Every direct dependency has version, source, purpose and detected license.
- Unknown/review-required licenses block according to policy.
- Release manifest can reference SBOM and provenance artifacts.

### Fast gate

- Run SBOM/license generation on minimal workspace.
- Test policy failure with a fixture dependency.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M0-T08 — Build deterministic multi-origin fixture and test harness foundations

**Primary role:** quality  
**Dependencies:** M0-T01  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Create local HTTP/HTTPS fixture server with multiple origins, redirects, DNS names and WebSocket support.
- Define test manifest, seeds, artifact layout and reproduction command.
- Add one fixture for navigation, DOM mutation, form action and network policy.

### Acceptance criteria

- Tests do not require third-party Internet.
- Fixture versions/hashes appear in results.
- A failed fixture produces a minimal command and trace reference.

### Fast gate

- Run fixtures on CI Linux.
- Verify origin/redirect/certificate behavior.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M0-T09 — Implement telemetry, correlation, redaction and evidence primitives

**Primary role:** platform + observability  
**Dependencies:** M0-T04, M0-T06  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Create typed metrics/log/trace context and correlation IDs.
- Implement centralized classification/redaction interface with canary tests.
- Define task/test evidence manifest and artifact hash helpers.

### Acceptance criteria

- No raw secret fixture appears in logs or evidence.
- Command/session/task IDs connect a sample trace.
- Evidence manifest is deterministic and validates referenced hashes.

### Fast gate

- Run redaction negative tests.
- Generate and validate a sample evidence bundle.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M0-T10 — Create fair benchmark harness and baseline corpus manifest

**Primary role:** performance + quality  
**Dependencies:** M0-T01, M0-T08, M0-T09  
**Risk:** medium  
**May run in parallel:** yes

### Deliverables

- Implement workload manifest, runner interface, resource sampling and verified postconditions.
- Record reference hardware/environment and cache/network controls.
- Create initial deterministic extraction, JavaScript, form and concurrency workloads.

### Acceptance criteria

- Runner counts only verified tasks as success.
- CPU, memory, latency, failures and retries are captured.
- A result can be reproduced from manifest and build identifiers.

### Fast gate

- Run a small Chromium baseline twice and compare variance.
- Verify failed postcondition is not counted as throughput.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M0-T11 — Deliver reproducible local full-stack development environment

**Primary role:** platform  
**Dependencies:** M0-T01, M0-T08, M0-T09  
**Risk:** medium  
**May run in parallel:** yes

### Deliverables

- Create Compose services for PostgreSQL, Redis-compatible store, S3-compatible store, fixture server and observability development endpoints.
- Provide local certificates, seed data and safe `.env.example`.
- Add start, stop, reset and health commands.

### Acceptance criteria

- Full local dependencies start from a clean checkout.
- Reset cannot target non-local endpoints.
- Health/doctor identifies missing or incompatible service.

### Fast gate

- Run local start/health/reset in clean environment.
- Scan generated environment/logs for secrets.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M0-T12 — Rehearse the two-agent autonomous loop and approve M0 exit

**Primary role:** orchestrator + reviewer  
**Dependencies:** M0-T02 through M0-T11  
**Risk:** high  
**May run in parallel:** no

### Deliverables

- Assign two non-overlapping small tasks to different supported tools/worktrees.
- Exercise claim, heartbeat, handoff, independent review, merge queue and state update.
- Produce M0 exit evidence and issues for workflow defects.

### Acceptance criteria

- No overlapping edit or lost state occurs.
- A fresh agent resumes from handoff without prior chat.
- Main fast gate is green and M1 ready tasks are generated.

### Fast gate

- Run the complete autonomous loop twice.
- Review claim/merge/evidence audit trail.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.
