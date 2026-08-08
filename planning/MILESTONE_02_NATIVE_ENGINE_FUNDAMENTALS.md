---
title: "M2 — Native Engine Fundamentals"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Program and Architecture"
purpose: "Provide executable task packets and exit criteria for milestone M2."
---

# M2 — Native Engine Fundamentals

## Objective

Create the first useful native browser path: streaming network and HTML, compact DOM, V8, event loop, navigation, selectors, events, fetch/storage and machine-oriented extraction.

## Entry criteria

- M1 compatibility-first platform passes its exit suite.
- Engine contract, fixtures, telemetry and router are available.

## Exit criteria

- Selected pages complete navigation, JavaScript and extraction natively.
- Native worker uses the same command/event/error model as Chromium.
- Unsupported behavior is explicit and fallback is operational.

## Scheduling notes

The orchestrator may run at most two implementation tasks concurrently. Parallel tasks must have non-overlapping write scopes and no unmerged shared-contract dependency. Every task follows `agents/AUTONOMOUS_LOOP.md` and records deferred heavy validation.

## Tasks

## M2-T01 — Implement native engine session, context, page and resource accounting

**Primary role:** native-engine  
**Dependencies:** M1-T03, M1-T04, M0-T04  
**Risk:** critical  
**May run in parallel:** constrained

### Deliverables

- Create native engine composition and canonical `EngineSession` facade.
- Implement context/page identities, lifecycle, cancellation and bounded resource counters.
- Expose capability snapshot and health to worker/scheduler.

### Acceptance criteria

- Create/close/cancel transitions match canonical contract.
- Every page resource category has accounting and hard-limit behavior.
- No protocol or control-plane types enter native core.

### Fast gate

- Run engine contract lifecycle tests.
- Run cancellation and forced budget-exhaustion tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M2-T02 — Implement native URL, DNS, TLS and HTTP streaming loader

**Primary role:** native-engine + security  
**Dependencies:** M2-T01, M0-T08, M0-T06  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Build URL normalization, request pipeline, connection pooling and streaming response API.
- Integrate deadlines, cancellation, request/byte budgets and network-policy callback.
- Support HTTP/1.1 and initial HTTP/2 using approved dependency/design.

### Acceptance criteria

- Fixture navigation handles redirects, compression, chunking and cancellation.
- Private/invalid destinations are rejected through policy hook.
- Bodies stream without mandatory full buffering.

### Fast gate

- Run multi-origin network fixtures and malformed response tests.
- Run focused SSRF/redirect negative fixtures.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M2-T03 — Implement streaming HTML tokenizer

**Primary role:** native-engine  
**Dependencies:** M2-T01, M0-T08  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Implement tokenizer states, character references, comments, doctypes, script/raw-text modes and bounded input.
- Expose incremental tokens and parse diagnostics without panics.
- Add fuzz target and curated malformed corpus.

### Acceptance criteria

- Priority tokenizer WPT/fixtures pass.
- Arbitrary chunk boundaries produce equivalent token stream.
- Adversarial input remains bounded and crash free in focused run.

### Fast gate

- Run tokenizer fixture/WPT shard.
- Run sanitizer/fuzz seed smoke.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M2-T04 — Implement HTML tree builder and document construction

**Primary role:** native-engine  
**Dependencies:** M2-T03, M2-T05 (interface coordination)  
**Risk:** critical  
**May run in parallel:** constrained

### Deliverables

- Implement insertion modes, foster parenting, templates, foreign content and parser/document integration.
- Create DOM nodes through a narrow builder interface.
- Track parser-blocking script checkpoints and errors.

### Acceptance criteria

- Priority tree-construction fixtures match normalized reference DOM.
- Malformed nesting recovers deterministically.
- Parser can pause/resume around script checkpoints.

### Fast gate

- Run tree-construction WPT subset and differential fixtures.
- Run deep/adversarial nesting limit tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M2-T05 — Implement compact DOM nodes, handles, mutation and lifecycle

**Primary role:** native-engine  
**Dependencies:** M2-T01, M0-T04  
**Risk:** critical  
**May run in parallel:** constrained

### Deliverables

- Create arena/region node storage, interned names and generational handles.
- Implement core node/document/element/text/fragment APIs and mutation invariants.
- Add wrapper invalidation hooks, mutation revisions and bulk document teardown.

### Acceptance criteria

- Stale/cross-document handles fail safely.
- Insert/remove/replace/adopt/clone maintain tree invariants.
- Document teardown releases accounted memory without lingering references.

### Fast gate

- Run DOM mutation/property tests.
- Run stale-handle and repeated create/destroy memory smoke.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M2-T06 — Build the narrow C++ V8 bridge and safe Rust facade

**Primary role:** native-engine + security  
**Dependencies:** M0-T01, M2-T01  
**Risk:** critical  
**May run in parallel:** constrained

### Deliverables

- Integrate pinned V8 artifact and create process platform/isolate/context opaque ABI.
- Implement exception-safe create/execute/terminate/dispose functions and Rust ownership wrappers.
- Add sanitizer build and ABI/version compatibility checks.

### Acceptance criteria

- Simple scripts execute and exceptions return typed diagnostics.
- Cross-thread/stale handle operations fail in checked tests.
- No C++ exception or raw pointer escapes the ABI.

### Fast gate

- Run bridge unit tests under sanitizer build.
- Run isolate/context repeated lifecycle and termination tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M2-T07 — Create V8 startup snapshot and browser binding scaffold

**Primary role:** native-engine  
**Dependencies:** M2-T06, M2-T05  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Generate deterministic custom startup snapshot with stable browser global scaffolding.
- Implement native handle wrapper template and initial Document/Node/Element bindings.
- Verify snapshot/build flags and fallback initialization behavior.

### Acceptance criteria

- Snapshot-loaded context exposes expected globals and DOM wrappers.
- Mismatched snapshot is rejected explicitly.
- Warm isolate/context startup baseline is recorded.

### Fast gate

- Run snapshot generation/hash/load tests.
- Benchmark cold versus snapshot startup on reference host.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M2-T08 — Implement web event loop, tasks, microtasks and timers

**Primary role:** native-engine  
**Dependencies:** M2-T06, M2-T07, M2-T02  
**Risk:** critical  
**May run in parallel:** constrained

### Deliverables

- Create task sources, owning execution lane, completion messages and bounded queues.
- Integrate V8 microtask draining, promises, setTimeout/setInterval and monotonic deadlines.
- Propagate cancellation/script termination and fairness accounting.

### Acceptance criteria

- Task/microtask/timer order passes focused fixtures.
- Infinite microtask/timer patterns hit typed budgets instead of starving cancellation.
- Network completion resumes on owning execution lane.

### Fast gate

- Run ordering and fake-clock tests.
- Run cancellation/starvation negative tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M2-T09 — Integrate navigation lifecycle, parser, scripts and history skeleton

**Primary role:** native-engine  
**Dependencies:** M2-T02, M2-T04, M2-T07, M2-T08  
**Risk:** critical  
**May run in parallel:** no

### Deliverables

- Implement top-level navigation state machine and document replacement.
- Stream response into parser, pause for scripts and resume event loop.
- Emit canonical navigation/document lifecycle and basic current URL/history state.

### Acceptance criteria

- Fixture reaches response, parsing, DOM interactive, task-ready/load states in valid order.
- A superseded navigation cannot satisfy a later wait.
- Cancel/redirect/script error produce classified outcomes.

### Fast gate

- Run navigation lifecycle fixture suite.
- Run rapid supersede/cancel/redirect tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M2-T10 — Implement CSS selector queries and initial XPath

**Primary role:** native-engine  
**Dependencies:** M2-T05  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Implement selector parser/matcher/query indexes for priority selectors.
- Add initial XPath evaluator required by extraction/automation.
- Expose canonical query outcomes with invalid/ambiguous/stale errors.

### Acceptance criteria

- Priority selector WPT/fixtures pass.
- Query order and live document revisions are correct.
- Invalid expressions never crash or silently match.

### Fast gate

- Run selector/XPath fixtures and differential checks.
- Run parser fuzz seed smoke.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M2-T11 — Implement event dispatch, focus and basic input model

**Primary role:** native-engine  
**Dependencies:** M2-T05, M2-T08  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Implement EventTarget/listeners, capture/target/bubble, cancelation and composed path foundation.
- Implement active element, focus/blur and basic keyboard/mouse event synthesis.
- Connect DOM mutation/lifecycle to event cleanup.

### Acceptance criteria

- Event phase/order/cancel tests pass.
- Focus transitions and detached target behavior are deterministic.
- Synthetic action reports dispatched/default/postcondition state.

### Fast gate

- Run event/focus WPT subset.
- Run listener mutation and detached-node negative tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M2-T12 — Implement fetch, XHR, cookies and web storage foundations

**Primary role:** native-engine  
**Dependencies:** M2-T02, M2-T07, M2-T08, M2-T09  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Expose fetch/Request/Response/Headers/Abort and XHR subset through bindings.
- Implement context cookie jar and local/session storage with quotas.
- Connect credentials, redirects, origin checks and events at initial P0 level.

### Acceptance criteria

- Fixture scripts can fetch JSON/text, abort, set/read cookies and storage.
- Cookie attributes/origin/storage isolation pass focused tests.
- Quota and network failures surface canonical/page errors correctly.

### Fast gate

- Run fetch/cookie/storage WPT subset.
- Run cross-origin/isolation/quota negative tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M2-T13 — Implement initial semantic index, markdown and structured extraction

**Primary role:** native-engine  
**Dependencies:** M2-T05, M2-T10, M2-T11  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Derive basic roles, accessible names, interactive elements, headings, links and forms.
- Generate bounded markdown and metadata/schema extraction from the live DOM.
- Attach document/semantic revisions and stable handles.

### Acceptance criteria

- Reference fixtures produce expected semantic nodes and readable markdown.
- Outputs are bounded/paginated and do not require duplicate DOM copies.
- Mutation invalidates/recomputes affected results correctly.

### Fast gate

- Run semantic/extraction fixture snapshots.
- Run large-document memory/output limit tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M2-T14 — Integrate native worker and pass the first native corpus gate

**Primary role:** orchestrator + quality  
**Dependencies:** M2-T01 through M2-T13, M1-T09  
**Risk:** critical  
**May run in parallel:** no

### Deliverables

- Package native worker with scheduler registration and canonical adapter.
- Enable router `prefer-native` for declared foundation capabilities.
- Run selected deterministic and approved corpus tasks, recording fallback/unsupported.

### Acceptance criteria

- At least the milestone target fixtures navigate, execute JS, query/extract and close natively.
- Unsupported capabilities return structured miss and can route to Chromium when policy allows.
- Native/Chromium traces share the canonical outcome format.

### Fast gate

- Run native engine contract and M2 corpus suite.
- Run worker crash/cancel/resource-limit integration tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.
