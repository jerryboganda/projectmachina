# Design: M2-T11 — Event Dispatch, Focus, and Basic Input Model

> Produced by a wave-2 architect research agent. Read-only; no files changed.

Target crate `crates/events` (currently `.gitkeep` only, not a workspace member — same bootstrap pattern as `crates/dom`). Builds on `.agent-state/design/M2-T05-dom-design.md` §2-3 and the error-taxonomy findings in `.agent-state/design/M2-M1-contract-compatibility-checklist.md`.

## 0. Crate-boundary decisions

`crates/events` depends only on `machina-dom` (path dep) — no `unsafe`/no `unwrap` on caller-reachable paths, hand-written errors, matches the workspace convention. **Not** dependent on `event-loop` (dispatch itself is synchronous per spec — a plain function call, not a task; scheduling *when* to call it is the integration layer's job, not this crate's) or `runtime-v8` (listener callback is a generic trait object, not a V8 handle — see §1, same seam discipline `WrapperObserver` established).

**Coordination note for `crates/dom`:** this design needs a stable, generation-independent key (`NodeIndex`) to index a per-node side table — if M2-T05 doesn't expose one, request a small additive accessor (`pub fn index(&self) -> NodeIndex`), not a redesign.

## 1. EventTarget/listener storage

Listeners live in a side-table owned by `crates/events`, not inside `dom::NodeData` — symmetric with M2-T05's own "stays agnostic" ethos.

```rust
pub struct ListenerId(u64);        // caller's own bookkeeping handle
pub struct CallbackIdentity(u64);   // opaque add/remove-matching key (e.g. a JS function's persistent-handle id)
pub trait EventListener: Send { fn handle_event(&self, event: &mut Event) -> ListenerResult; } // zero V8 types
pub enum ListenerResult { Completed, Threw } // Threw is a forward-compat hook, always Completed today (no JS runtime yet)
struct ListenerEntry { id, event_kind: EventKind, capture: bool, once: bool, passive: bool, identity: CallbackIdentity, callback: Box<dyn EventListener> }
pub struct EventTargetRegistry { document: DocumentId, by_node: HashMap<NodeIndex, Vec<ListenerEntry>>, focus: FocusState, next_listener_id: u64 }
```

`by_node` keyed by `NodeIndex` within a table scoped to one `DocumentId`; every public method calls `document.resolve(handle)` first, so wrong-document/stale/freed handles are rejected by `dom` before the table is touched, for free. `add_event_listener` is idempotent per spec (linear scan on `(event_kind, capture, identity)`, small-vec philosophy matching `AttributeMap`). `CallbackIdentity` (opaque comparison key) exists specifically so `crates/events` never needs `Box<dyn EventListener>: PartialEq` and never needs to know what "a JS function" is — `runtime-v8` decides what identity means.

## 2. Dispatch algorithm

`Event{kind, bubbles, cancelable, composed, target, current_target, phase, path: Vec<NodeHandle> (snapshot), default_prevented, propagation_stopped, immediate_propagation_stopped, in_passive_listener, is_trusted}` with `prevent_default()`/`stop_propagation()`/`stop_immediate_propagation()`/`composed_path()`.

`dispatch_event(document, registry, target, init)`:
1. `document.resolve(target)?` → `TargetNotFound`.
2. **Build `path` once, up front** — walk `parent` links root→target, bounded by the same guard style as `dom`'s ancestor-walk safety. This is a **snapshot, not a live walk** — if a listener mutates the tree mid-dispatch, remaining phases still iterate the originally-computed path (matches real DOM semantics: propagation path is fixed at dispatch start).
3. **Capture**: for each ancestor root→parent-of-target: re-validate `resolve()` (skip that node's listeners if it no longer resolves, don't fail the whole dispatch); snapshot the node's listener list before invoking (so add/remove-during-dispatch is spec-correct: added-during-this-dispatch doesn't fire, removed-before-its-turn doesn't fire); invoke `capture:true` listeners in registration order; check `immediate_propagation_stopped` after each, `propagation_stopped` after the node.
4. **Target**: invoke all listeners on target (both capture flags — spec doesn't distinguish at-target).
5. **Bubble** (if `event.bubbles`): reversed ancestor order, `capture:false` only, same re-validate/snapshot pattern.
6. `once` listeners removed at invocation time; `passive` listeners run with `in_passive_listener=true` so `prevent_default()` no-ops without throwing.
7. Returns `DispatchOutcome{default_prevented, phases_completed}` — distinguishes normal completion from "target/ancestor went stale mid-dispatch, some phases skipped."

**Composed-path/M3-T04 boundary, stated explicitly:** `path` exists now as the flat light-tree ancestor chain (no `ShadowRoot` NodeKind yet per M2-T05 §7). M3-T04's job is scoped precisely to changing *how `path` is constructed* (shadow-aware) and adding retargeting — it should not need to touch `dispatch.rs`'s phase/order/cancellation algorithm at all.

## 3. Focus model

Folded into `EventTargetRegistry` (shares construction/lifecycle hooks). `focus(document, target)`: no-op with no events if already active; sets `active_element` **before** firing events (deliberate, matches observable Chromium behavior — exact WPT edge-case ordering explicitly deferred past M2-T11); fires `blur`+`focusout` at old (if it still resolves — skip if already freed), then `focus`+`focusin` at new. `blur()`: fires `blur`+`focusout` at old, sets `active_element = None` — no fallback-to-body (explicitly deferred, non-breaking to add later).

**Minimal focusability**: parseable `tabindex` (any integer incl. negative) OR built-in tag set (`a[href]`, `button`, `input`, `select`, `textarea`, `area[href]`, `iframe`) — excludes visibility/geometry/disabled/`pointer-events` (M3-T12 territory) and Tab-key sequential navigation. Explicit minimal bar for "basic input model," not spec-complete.

**Interop with DOM lifecycle — three hooks matching `WrapperObserver`'s signatures verbatim** (so the composing owner's single observer impl can simply forward):
- `handle_node_changed(handle, Detached)` where `handle == active_element`: fires `blur`+`focusout` (legal — still resolvable, merely detached per §6), then clears to `None`. Does NOT pick a next focus target (out of scope, documented simplification). Ignored for other changes/non-active nodes.
- `handle_node_freed(handle)`: removes `by_node[handle.index()]` entirely (drops every listener box, running `Drop` — the GC-safety seam a V8-backed listener needs to release its persistent handle). If `handle == active_element`, clears silently **without** firing (handle already stale, dispatch impossible) — belt-and-suspenders for any path where Freed arrives without a preceding Detached.
- `handle_document_teardown()`: clears everything in one O(1) drop, fires **no** synthetic events (mirrors `WrapperObserver`'s "fires once, not per-node" contract).

This directly satisfies the "focused element detached / document torn down must not leave dangling focus state" requirement — no panic, no double-fire.

## 4. Synthetic events for `interaction.click.v1`

`MouseEventInit{button, buttons, detail, client_x/y, modifier keys}`; `KeyboardEventInit` present now too (unused by click.v1, avoids a later crate redesign for `interaction.type.v1`). `ClickPayload` (schema) only carries `selector: string` — `perform_click` synthesizes defaults (`Main` button, `detail:1`, coords `0.0`, no modifiers) — deliberate, explicitly scoped, not an oversight; real coordinates need M3-T12 geometry.

`perform_click(document, registry, target)`:
1. `resolve(target)?` → `TargetNotFound`.
2. **Interactability precheck, deliberately narrow**: target must be attached (ancestor chain reaches `Document`) → else `NotInteractable`. This is the *only* rule enforced (visibility/size/occlusion/disabled/`pointer-events` are explicit, called-out gaps needing M3-T12).
3. Dispatch `mousedown` (bubbles/cancelable).
4. If not default-prevented and target/nearest-focusable-ancestor differs from active element → `focus()`. Else `focus: None`.
5. Dispatch `mouseup`.
6. Dispatch `click`. **No further default action** (link-nav/form-submit belong to M2-T09/forms work) — explicit boundary so future readers don't expect navigation from this crate.
7. Re-validate `resolve(target)` before each step 3/5/6 — if a listener freed it mid-dispatch, remaining phases skip and `postcondition = Failed("target freed during dispatch")` (genuine, actionable, not swallowed).
8. `postcondition = Verified` whenever all three events fully dispatch, regardless of engine-level `preventDefault()` calls — **prevented-default is a default-action outcome, not a postcondition failure**, deliberately not conflated (`dispatched.*_default_prevented` carries that separately).

**Mapping to `CanonicalErrorCode` (no new codes, per the checklist's finding)** — done by the native-core wiring layer, not this crate: `TargetNotFound`→`ELEMENT_NOT_FOUND`, `NotInteractable`→`ELEMENT_NOT_INTERACTABLE`, `Failed(_)` postcondition→`ACTION_POSTCONDITION_FAILED`. Selector→`NodeHandle` resolution (and `ELEMENT_AMBIGUOUS`) happens entirely in M2-T10's query API *before* calling into `crates/events` — this crate only ever receives an already-resolved handle. `crates/events` stays free of `serde`/JSON — returns typed Rust structs, shaping into `CommandOutcome.result` is native-core's job.

**Cross-task risk flagged explicitly:** milestone doc lists M2-T11's deps as M2-T05+M2-T08 only, NOT M2-T10 — but `interaction.click.v1` needs M2-T10's selector resolution to be useful end-to-end. The M2-T11 builder must check `WORK_QUEUE.md` for M2-T10's actual landing status at claim time rather than assume it; `perform_click` itself has no hard dependency (takes a pre-resolved handle), but the native-core wiring arm and the required smoke-fixture update (§7) do.

## 5. Cleanup guarantee

`EventTargetRegistry`'s three hook methods match `WrapperObserver`'s exactly, so the composing owner's single observer impl (native-core/V8 bridge — `crates/events` doesn't claim the single observer slot itself, since other concerns like V8 wrapper GC will also need it) simply forwards. Correctness argument: since `dom` guarantees `on_node_freed`/`on_document_teardown` fire **exactly once** per node/document (M2-T05 §3, backed by its own tests), and `crates/events`' table removal happens synchronously inside that same call, there's no window where a listener outlives its node's free, and no double-cleanup path. `crates/events` needs an equivalent regression test proving it doesn't defeat dom's "one call for bulk teardown" guarantee.

## 6. Detached-target dispatch — allowed, not rejected

Generic primitives (`add_event_listener`/`dispatch_event`) only distinguish "resolvable" (attached or detached-but-live) from "stale" (freed/wrong-doc/closed) — no stronger "must be connected" gate. **Justification:** matches real browsers exactly — `EventTarget.addEventListener()`/`dispatchEvent()` have no "must be connected" precondition in spec or any implementation (`document.createElement` immediately supports listeners before insertion). Where this IS restricted: `perform_click` (§4) adds its own narrower, action-specific attachment gate on top — a business rule for "can a user meaningfully click this," not a change to `EventTarget` semantics. Called out explicitly so a future reader doesn't try to "fix" the generic primitive to match the stricter click rule.

## 7. Test strategy → acceptance criteria

Phase/order/cancel → `tests/dispatch_order.rs` (capture→target→bubble order, per-node registration-order preservation, stopPropagation/stopImmediatePropagation semantics, preventDefault on cancelable/passive, once-fires-once, add/remove-during-dispatch snapshot semantics, non-bubbling skips bubble, composed_path is a dispatch-start snapshot unaffected by mid-dispatch mutation — explicit regression test doubling as M3-T04 foundation evidence). Focus determinism → `tests/focus.rs` (ordering, no-op-if-already-active, detach/free/teardown lifecycle interop, all per §3). Detached-target behavior → `tests/detached_dispatch.rs` (listener+dispatch on never-attached and detached-after-attach both succeed; freed/stale handle → typed error not panic; `perform_click` on detached → `NotInteractable`, explicit contrast proving the two-tier §6 decision). Synthetic action reporting → `tests/click_action.rs` (happy path Verified; mousedown-preventDefault suppresses implicit focus; click-preventDefault surfaces separately from postcondition=Verified; mid-dispatch target-freeing → graceful Failed postcondition, not panic). Cleanup → `tests/cleanup.rs` (Drop-counting listener double proves no leak, one `handle_document_teardown` call not per-node, re-attachment preserves listeners).

**Required `m1-compatibility-smoke.mjs` update (per the M1/M2 checklist's finding #4):** once wired, `"#missing"` selector now resolves to `ELEMENT_NOT_FOUND`, not `UNSUPPORTED_CAPABILITY` — the eventual builder's PR must change that assertion (or add a distinct positive-path test) and explicitly note this is a deliberate, reviewed behavior change from the M1 baseline, per the checklist's requirement.

## 8. Module layout

```
crates/events/  Cargo.toml (machina-dom only — no serde/runtime-v8/command-model/command-bus/session/event-loop)
  src/ lib.rs · listener.rs (EventListener trait, ListenerId, CallbackIdentity) ·
       target.rs (EventTargetRegistry, WrapperObserver-shaped forwarding methods) ·
       event.rs (Event, EventKind open enum, EventPhase) · dispatch.rs (capture/target/bubble algorithm) ·
       focus.rs (focus/blur, is_focusable_by_default) · mouse.rs · keyboard.rs (present, unused by click.v1) ·
       action.rs (perform_click, ClickOutcome, PostconditionState) · error.rs (EventError, hand-written)
  tests/ dispatch_order.rs · focus.rs · detached_dispatch.rs · click_action.rs · cleanup.rs
```
Workspace wiring: add `"crates/events"` to root members, `[dependencies]` limited to `machina-dom`; flagged as one of the tasks that could apply the `check-boundaries.mjs` fix if a prior task hasn't already.

## Files reviewed

`.agent-state/design/M2-T05-dom-design.md` · `.agent-state/design/M2-M1-contract-compatibility-checklist.md` · `planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md` (T05/T08/T09/T10/T11/T12) · `scripts/test/m1-compatibility-smoke.mjs` · `crates/command-model/src/generated.rs` (`CanonicalErrorCode`) · `schemas/command-model/v0.1/command-model.json` (`ClickPayload`) · `crates/native-core/src/lib.rs` · `crates/capability/src/lib.rs` · root `Cargo.toml`.
