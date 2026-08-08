---
title: "M3 — Native Web APIs and Automation"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Program and Architecture"
purpose: "Provide executable task packets and exit criteria for milestone M3."
---

# M3 — Native Web APIs and Automation

## Objective

Expand the native engine from extraction fundamentals into dependable browser automation: forms, verified actions, frames, Shadow DOM, workers, storage, interception, semantic visibility and migration-ready state.

## Entry criteria

- M2 native fundamentals pass.
- Capability registry and router can enable native features incrementally.

## Exit criteria

- Target agent/extraction corpus reaches useful native coverage with explicit fallback.
- State export/action checkpoints are ready for migration.
- No silent unsupported or unsafe action semantics in enabled capabilities.

## Scheduling notes

The orchestrator may run at most two implementation tasks concurrently. Parallel tasks must have non-overlapping write scopes and no unmerged shared-contract dependency. Every task follows `agents/AUTONOMOUS_LOOP.md` and records deferred heavy validation.

## Tasks

## M3-T01 — Implement forms, controls, validation and default actions

**Primary role:** native-engine  
**Dependencies:** M2-T11, M2-T12  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Implement text controls, checkbox/radio, select/options, buttons, labels, fieldsets and form association.
- Implement value/checked/selected state, input/change events, validation subset and submission encoding.
- Integrate navigation/fetch default actions and semantic state.

### Acceptance criteria

- Representative forms can be filled, selected and submitted with correct events/payload.
- Disabled/required/invalid controls behave according to focused standards tests.
- Form state survives/exports according to lifecycle policy.

### Fast gate

- Run forms WPT/fixture subset.
- Run event/default-action differential tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M3-T02 — Implement verified keyboard, mouse, click, fill, select and check actions

**Primary role:** native-engine + agent-runtime  
**Dependencies:** M2-T10, M2-T11, M3-T01  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Implement semantic/DOM locator resolution and action preconditions.
- Add keyboard/mouse sequence and control-specific operations.
- Verify postconditions, revisions, navigation and typed failures.

### Acceptance criteria

- Action success requires attached/visible/enabled/stable state and postcondition.
- Ambiguous/detached/hidden/disabled cases return precise errors.
- Side-effect class and replay metadata are emitted.

### Fast gate

- Run action fixture matrix and reference differential.
- Run mutation-during-action and duplicate side-effect tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M3-T03 — Complete history, same-document navigation and task-ready waits

**Primary role:** native-engine  
**Dependencies:** M2-T09, M2-T10, M2-T13  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Implement history entries, push/replace state, popstate/hashchange and back/forward/reload.
- Implement selector, function, semantic-stable and bounded network-idle waits.
- Associate waits with navigation/document identity and cancellation.

### Acceptance criteria

- Same-document and cross-document events/order pass fixtures.
- Wait cannot be satisfied by superseded/stale document.
- Every wait is deadline/cancellation aware and reports observed revisions.

### Fast gate

- Run history/navigation WPT subset.
- Run stale-wait and never-idle resource tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M3-T04 — Implement Shadow DOM, slots and custom elements

**Primary role:** native-engine  
**Dependencies:** M2-T05, M2-T07, M2-T08, M2-T11  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Implement shadow roots, slots/composed tree and event retargeting.
- Implement custom element registry, upgrade and reaction queue.
- Extend selectors/semantic traversal with explicit closed-root policy.

### Acceptance criteria

- Priority Shadow DOM/custom-element WPT passes.
- Composed events and slot changes update semantic revisions.
- Closed-root automation behavior is documented and capability-gated.

### Fast gate

- Run WPT subset and composed-tree differential fixtures.
- Run reaction recursion/budget tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M3-T05 — Implement frames and browsing contexts

**Primary role:** native-engine  
**Dependencies:** M2-T09, M2-T12, M3-T04  
**Risk:** critical  
**May run in parallel:** constrained

### Deliverables

- Create frame tree, child documents, navigation, lifecycle and context IDs.
- Implement same-origin access and cross-origin boundary checks.
- Integrate events, semantic output, resource limits and protocol mapping.

### Acceptance criteria

- Same-origin and cross-origin fixture behavior matches declared semantics.
- Frame removal/navigation invalidates handles and waits safely.
- Frame count/resource budgets are enforced.

### Fast gate

- Run frame WPT/fixture subset.
- Run cross-origin access and frame-bomb limits.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M3-T06 — Harden CORS, referrer, cache, redirects and network semantics

**Primary role:** native-engine + security  
**Dependencies:** M2-T02, M2-T12  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Implement prioritized CORS/preflight, referrer policy, redirect credential/header behavior and cache semantics.
- Add response streaming/backpressure and content decoding edge cases.
- Expand timing/error/capability instrumentation.

### Acceptance criteria

- Priority fetch/CORS/referrer/cookie tests pass.
- Cross-origin failures match page-visible and canonical behavior.
- Cache/redirect cannot bypass network policy or leak tenant identity.

### Fast gate

- Run network WPT shard and policy fixtures.
- Run malformed/oversized/decompression bomb tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M3-T07 — Implement WebSocket lifecycle and limits

**Primary role:** native-engine  
**Dependencies:** M2-T02, M2-T08, M2-T07  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Implement handshake, messaging, close/error events, binary/text and backpressure.
- Apply destination policy, message/byte/time limits and cancellation.
- Integrate canonical network events and resource accounting.

### Acceptance criteria

- Echo/multi-message/close fixtures pass.
- Policy denial and oversized/slow channel terminate correctly.
- Connections close on session end and do not leak across contexts.

### Fast gate

- Run WebSocket fixture matrix.
- Run backpressure/idle/budget negative tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M3-T08 — Implement dedicated and shared worker primitives

**Primary role:** native-engine  
**Dependencies:** M2-T06, M2-T08, M2-T12  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Create worker execution contexts, message passing, structured clone subset and termination.
- Apply origin, network, storage and resource policy.
- Integrate worker lifecycle/events and crash containment.

### Acceptance criteria

- Worker script/message fixtures pass with deterministic termination.
- Worker limits and session cancellation propagate.
- No worker accesses another context or host capability.

### Fast gate

- Run worker WPT subset and messaging fixtures.
- Run worker storm/resource isolation tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M3-T09 — Implement prioritized service worker and IndexedDB capabilities

**Primary role:** native-engine  
**Dependencies:** M3-T08, M3-T06  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Implement minimum registration/lifecycle/fetch interception required by target corpus.
- Implement versioned IndexedDB storage abstraction and prioritized operations.
- Make unsupported portions explicit and fallback-aware.

### Acceptance criteria

- Selected corpus service-worker/IndexedDB journeys pass natively or route explicitly.
- Origin/profile isolation and quotas are enforced.
- Restart/persistence behavior matches declared profile policy.

### Fast gate

- Run selected WPT and corpus fixtures.
- Run quota/corruption/restart negative tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M3-T10 — Implement network interception, proxy authentication and routing hooks

**Primary role:** native-engine + protocol  
**Dependencies:** M3-T06, M1-T09  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Expose canonical request pause/continue/modify/fulfill/fail operations.
- Support scoped proxy references/auth and DNS mode.
- Integrate deadlines, bounded bodies, audit and protocol adapter hooks.

### Acceptance criteria

- Fixture requests can be observed/modified/fulfilled deterministically.
- Proxy credentials never appear in logs/events.
- Slow/missing interceptor cannot deadlock the session.

### Fast gate

- Run interception/proxy fixture suite.
- Run timeout, oversized fulfill and canary secret tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M3-T11 — Implement controlled uploads, downloads, dialogs and popups

**Primary role:** native-engine + security  
**Dependencies:** M3-T01, M3-T05, M3-T06  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Implement virtual file upload handles and isolated download artifacts.
- Implement alert/confirm/prompt events and policy-driven response.
- Implement popup/new-context lifecycle with quotas and opener policy.

### Acceptance criteria

- Page never receives arbitrary host path.
- Downloads are bounded, isolated, scanned/policy checked and exported as artifacts.
- Unattended dialog/popup policy is explicit and cannot hang indefinitely.

### Fast gate

- Run upload/download/dialog/popup fixtures.
- Run path traversal, oversized file and popup storm tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M3-T12 — Implement semantic CSS, visibility, geometry and hit-testing subset

**Primary role:** native-engine  
**Dependencies:** M2-T13, M3-T04, M3-T05  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Implement prioritized cascade/inheritance and properties for display/visibility/opacity/pointer/focus/overflow.
- Compute simplified boxes, clipping, viewport intersection and hit target confidence.
- Expose limitation/confidence and route visual certainty to Chromium.

### Acceptance criteria

- Action visibility/interactability fixtures match declared reference behavior.
- Unsupported visual complexity does not produce false certainty.
- Incremental style/mutation invalidation updates semantic revisions.

### Fast gate

- Run semantic CSS/interaction differential suite.
- Run large stylesheet/selector resource tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M3-T13 — Implement semantic deltas and single-pass multi-output extraction

**Primary role:** native-engine + agent-runtime  
**Dependencies:** M3-T12, M2-T13  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Add revision window and insert/update/delete/reorder delta format.
- Update interactive index, markdown, forms, links, headings and schema outputs from shared traversal/indexes.
- Add pagination, truncation and full-snapshot-required behavior.

### Acceptance criteria

- Small mutations produce bounded deltas rather than full snapshots.
- All output views reference consistent document/semantic revisions.
- Expired revision returns explicit resync requirement.

### Fast gate

- Run delta sequence and output consistency tests.
- Benchmark full snapshot versus delta on large fixture.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M3-T14 — Implement native state export and verified action history

**Primary role:** native-engine + platform  
**Dependencies:** M3-T02, M3-T03, M3-T05, M3-T10  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Export allowlisted cookies/storage/config/current URL and origin-scoped state.
- Record deterministic action history, side-effect class and verified checkpoints with secret references.
- Produce versioned transfer bundle and redaction/integrity metadata.

### Acceptance criteria

- Bundle imports into test destination with declared omissions.
- Secret values never appear in action log/bundle metadata.
- Unsafe side-effect cannot be marked replayable without verified checkpoint.

### Fast gate

- Run export/import fixture and schema compatibility tests.
- Run canary secret and side-effect replay negative tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M3-T15 — Reach the target native automation and extraction coverage gate

**Primary role:** orchestrator + quality  
**Dependencies:** M3-T01 through M3-T14, M2-T14  
**Risk:** critical  
**May run in parallel:** no

### Deliverables

- Run milestone target corpus under native-only and prefer-native modes.
- Triage every failure to defect, unsupported capability, policy or test/site issue.
- Update router/capability registry and prioritize remaining gaps.

### Acceptance criteria

- Native fast path reaches the M3 agreed target on stable selected corpus.
- Every non-native outcome has explicit reason and safe fallback behavior.
- No critical crash, silent no-op or side-effect replay defect remains.

### Fast gate

- Run M3 corpus, selected WPT and differential shards.
- Run short concurrency/resource soak.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.
