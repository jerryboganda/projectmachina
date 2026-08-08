---
title: "M4 — Protocols and SDKs"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Program and Architecture"
purpose: "Provide executable task packets and exit criteria for milestone M4."
---

# M4 — Protocols and SDKs

## Objective

Stabilize the public command model and make Project Machina usable through production-quality HTTP, gRPC, CDP, WebDriver BiDi, MCP and language SDK surfaces.

## Entry criteria

- M3 target native coverage and state foundations pass.
- Canonical engine behavior is stable enough to certify adapters.

## Exit criteria

- Command model v1 and compatibility tooling are established.
- Published client/protocol matrices are evidence-backed.
- All supported adapters fail explicitly and preserve policy, cancellation and engine metadata.

## Scheduling notes

The orchestrator may run at most two implementation tasks concurrently. Parallel tasks must have non-overlapping write scopes and no unmerged shared-contract dependency. Every task follows `agents/AUTONOMOUS_LOOP.md` and records deferred heavy validation.

## Tasks

## M4-T01 — Stabilize command model version one and compatibility tooling

**Primary role:** architect + protocol  
**Dependencies:** M3-T15, M1-T12  
**Risk:** critical  
**May run in parallel:** no

### Deliverables

- Review all implemented canonical commands/events/errors/capabilities and publish v1 schemas.
- Add compatibility diff tooling, reserved fields and migration rules.
- Generate Rust/TypeScript/Python and protobuf/OpenAPI projections from one source strategy.

### Acceptance criteria

- All enabled engine features map to v1 commands without transport leakage.
- A breaking schema fixture is rejected and additive change accepted.
- Generated outputs are deterministic and cross-language round trips pass.

### Fast gate

- Run full schema/descriptor compatibility and round-trip suite.
- Compile all generated consumers.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M4-T02 — Complete HTTP API version one

**Primary role:** protocol  
**Dependencies:** M4-T01, M1-T07, M1-T08  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Implement organizations/projects/policies/sessions/commands/events/workflows/artifacts/capabilities resources needed for beta.
- Add cursor pagination, ETags/expected version, idempotency and SSE resume.
- Generate OpenAPI and documentation/error examples.

### Acceptance criteria

- API contract passes positive/negative/auth/idempotency/pagination tests.
- No endpoint returns an engine-specific undocumented response.
- OpenAPI and server routing remain in sync.

### Fast gate

- Run HTTP contract suite and generated client smoke.
- Run malformed/oversized/slow-reader security tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M4-T03 — Complete gRPC commands, bidirectional events and backpressure

**Primary role:** protocol  
**Dependencies:** M4-T01, M1-T07, M1-T08  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Implement service methods and bidirectional frames for commands/cancel/ack/events/progress.
- Add bounded buffers, resume/resync and flow-control telemetry.
- Generate supported SDK stubs and gateway compatibility.

### Acceptance criteria

- Pipelined safe commands correlate correctly and unsafe commands serialize.
- Slow/disconnected clients cannot create unbounded server state.
- Deadlines/cancellation map exactly to canonical outcomes.

### Fast gate

- Run stream ordering/reconnect/backpressure suite.
- Run descriptor compatibility tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M4-T04 — Implement CDP Target, Browser, Page and Runtime domains

**Primary role:** protocol  
**Dependencies:** M4-T01, M3-T15  
**Risk:** critical  
**May run in parallel:** constrained

### Deliverables

- Generate dispatcher/types from pinned CDP schema and implement target/session mapping.
- Implement certified subset for Browser/Target/Page/Runtime including navigation, lifecycle, evaluate and exceptions.
- Expose explicit protocol errors and migration behavior.

### Acceptance criteria

- Direct CDP client can create context/page, navigate, evaluate and close.
- Target/session events remain coherent across supported lifecycle.
- Unknown/unsupported methods never return false success.

### Fast gate

- Run domain contract fixtures against native and Chromium.
- Run disconnect/reconnect/unsupported tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M4-T05 — Implement CDP DOM, Network, Input, Emulation and supporting domains

**Primary role:** protocol  
**Dependencies:** M4-T04, M3-T02, M3-T10, M3-T11  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Implement prioritized DOM query/describe, Network cookies/events/interception, Input and Emulation mappings.
- Add logs/dialogs/downloads/storage/permissions subset needed by clients.
- Mark native-limited and Chromium-only operations accurately.

### Acceptance criteria

- Certified domain commands map to canonical behavior and events.
- Protocol object IDs become invalid safely on document/context change.
- Chromium-only operation routes or errors according to session policy.

### Fast gate

- Run domain matrix and ID lifetime tests.
- Run network interception/input differential smoke.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M4-T06 — Certify initial Playwright and Puppeteer version matrix

**Primary role:** protocol + quality  
**Dependencies:** M4-T04, M4-T05  
**Risk:** critical  
**May run in parallel:** no

### Deliverables

- Select pinned supported client versions and build automated feature matrix.
- Test contexts/pages/frames/navigation/evaluate/locators/actions/storage/network/dialogs/downloads/parallelism.
- Publish pass/limited/Chromium-only/unsupported with evidence.

### Acceptance criteria

- Quick starts and target P0 workflows pass for every certified version.
- Known limitations produce explicit results, not hangs.
- Matrix identifies server/engine/client versions and isolation policy.

### Fast gate

- Run full initial client matrix.
- Run one previous/newer unsupported client to verify clear incompatibility behavior.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M4-T07 — Implement WebDriver BiDi session, browsing context and script modules

**Primary role:** protocol  
**Dependencies:** M4-T01, M3-T05  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Implement session status/subscribe/end, browsing context tree/create/navigate/reload/close and script realms/evaluate/call.
- Map canonical context/page/frame and V8/Chromium realms.
- Implement subscribed lifecycle/log/script events and typed errors.

### Acceptance criteria

- Direct BiDi client completes context/navigation/script journey natively and via Chromium.
- Event filters/order and realm invalidation pass fixtures.
- Unsupported standard commands fail in protocol-conformant form.

### Fast gate

- Run BiDi module fixtures and schema validation.
- Run context/realm lifecycle negative tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M4-T08 — Implement WebDriver BiDi network, storage and input modules

**Primary role:** protocol  
**Dependencies:** M4-T07, M3-T02, M3-T10, M3-T12  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Map prioritized network events/interception, cookies/storage and input actions.
- Add context filters, request IDs, action sequences and engine limitation metadata.
- Implement namespaced extensions only for Project Machina-specific semantic/fallback data.

### Acceptance criteria

- Target module fixtures pass with native/hybrid paths.
- Input and interception cannot bypass canonical policy.
- Extension fields do not change standard command semantics.

### Fast gate

- Run BiDi network/input/storage suite.
- Run authorization and malformed action tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M4-T09 — Integrate Selenium and WPT through WebDriver BiDi

**Primary role:** protocol + quality  
**Dependencies:** M4-T07, M4-T08  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Select Selenium client versions and create connect/session helpers.
- Connect WPT harness to BiDi/native worker and preserve expected result metadata.
- Publish module/client limitations and reproduction commands.

### Acceptance criteria

- Selenium sample suite completes target journeys.
- Applicable WPT shards run without custom behavior bypassing product contracts.
- Failures retain protocol, engine and test identifiers.

### Fast gate

- Run Selenium matrix smoke and representative WPT shard.
- Run session teardown/reconnect tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M4-T10 — Implement production MCP server and agent-native tools

**Primary role:** protocol + agent-runtime  
**Dependencies:** M4-T01, M3-T02, M3-T13  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Implement authenticated MCP transport and typed session/navigation/observe/action/workflow/diagnostic tools.
- Add bounded semantic snapshot/delta outputs, progress/cancellation and resource access.
- Apply prompt-injection separation, approvals, quotas and redaction.

### Acceptance criteria

- MCP client completes a multi-step semantic task with verified actions.
- No tool can change policy or reveal a secret because page text requests it.
- Large output truncates/paginates explicitly.

### Fast gate

- Run MCP protocol/tool schema and task suite.
- Run prompt-injection, auth and output-limit negative tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M4-T11 — Promote TypeScript and Python SDKs to production beta quality

**Primary role:** protocol  
**Dependencies:** M4-T02, M4-T03, M4-T10  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Complete ergonomic clients for sessions/pages/locators/events/workflows/artifacts/capabilities.
- Add packaging, docs, examples, retries, cancellation, cleanup and telemetry hooks.
- Run supported runtime/version matrix.

### Acceptance criteria

- Clean-environment quick starts finish within documented steps.
- Typed errors and stream recovery pass compatibility tests.
- Packages contain no generated drift or secret/test artifact.

### Fast gate

- Run package install and end-to-end examples.
- Run previous SDK/new server compatibility where promised.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M4-T12 — Deliver Go, Java and Rust SDKs and publish version policy

**Primary role:** protocol  
**Dependencies:** M4-T02, M4-T03  
**Risk:** medium  
**May run in parallel:** yes

### Deliverables

- Generate low-level clients and implement idiomatic session/page/error/event facade.
- Add package publishing metadata, examples and support matrix.
- Finalize deprecation/version compatibility policy across all SDKs.

### Acceptance criteria

- Each SDK completes session/navigation/extraction/cancel/close quick start.
- Language-native cancellation/resource cleanup works.
- Version matrix is generated from tested combinations.

### Fast gate

- Run clean install/build/examples for supported language versions.
- Run canonical error and event-stream tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.
