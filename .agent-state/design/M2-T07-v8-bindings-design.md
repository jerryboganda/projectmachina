# Design: M2-T07 — V8 Startup Snapshot and Browser Binding Scaffold

> Produced by a wave-1 architect research agent. Read-only; no code changes.
> Pairs with `.agent-state/design/M2-T06-v8-bridge-design.md` (the `crates/runtime-v8` /
> `cpp/v8-bridge` contract this task binds on top of) and
> `.agent-state/design/M2-T05-dom-design.md` (the `crates/dom` surface this task exposes to JS).
> T06 is being implemented in parallel on `agent/M2-T06-v8-bridge`; this doc treats its design
> doc as the contract, not its (possibly still-changing) code.

Scope source: `planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md` (`## M2-T07`). Target: a
new module area inside `crates/runtime-v8` (or a sibling crate, see open question OQ-1) plus
new C++ under `cpp/v8-bridge/` for snapshot creation and object-template binding. Depends on
M2-T06 (isolate/context/execute plumbing) and M2-T05 (`crates/dom`, merged — read directly).

## 0. Design summary

Two distinct artifacts, both required by the acceptance criteria:

1. **A deterministic startup snapshot** — a serialized V8 heap blob built once (offline/CI,
   not per-context) containing global scaffolding (`window`, `document` shape, prototype
   chains, `FunctionTemplate`/`ObjectTemplate` definitions) so warm context creation is fast
   and stable across processes. Content-hashed and version-pinned; a runtime refuses to load a
   snapshot that doesn't match its own build.
2. **A binding-generation layer** that, per live `Context`, instantiates real wrapper objects
   backed by `machina_dom::NodeHandle`s and wires them to native accessor/method callbacks.
   The snapshot bakes in *templates* (shape/behavior), never live document data — no page
   content, no `NodeHandle`, no anything document-specific goes into the snapshot itself.

The hard problem this doc has to solve is #2's handle lifetime: `machina_dom` handles are
`Copy` generational values with no `Drop`/no notion of "this is still referenced," while V8
wrapper objects are garbage-collected and can outlive the Rust-side node they wrap (page script
holds a `Node` reference after the app calls `removeChild` + the DOM crate frees the slot).
Section 3 is the answer.

## 1. What gets exposed to JS, and how

### 1.1 Global scaffolding (baked into the snapshot)

- `window` — the global object itself (not a property of it), per HTML semantics.
- `document` — a lazily-materialized accessor on `window` (see 1.3) that resolves to the
  wrapper for the current `Document`'s root once a context is bound to a live document.
- Constructor-shaped globals for exposed interfaces (`Node`, `Element`, `Text`, `Comment`,
  `DocumentFragment`, `DocumentType`) as `FunctionTemplate`-derived objects with the right
  prototype chain (`Element.prototype` inherits `Node.prototype`, etc.) — `instanceof` and
  prototype-method lookup work identically to a real browser, matching `NATIVE_ENGINE.md`'s
  "standards-oriented ... semantics" goal and unblocking future WPT work.
- No fetch/timer/event globals in T07's snapshot — `NATIVE_ENGINE.md`'s capability phases and
  `V8_INTEGRATION.md`'s "fetch/event/workflow primitives" line item place those in the
  snapshot eventually, but concretely they're M2-T08/T09/T12 deliverables. T07 reserves the
  template *slots* (so adding them later doesn't require a new snapshot layout/version bump for
  the reserved-name set) but implements only the DOM/Node/Element/Document surface actually in
  scope, per the acceptance criteria ("Document/Node/Element bindings").

### 1.2 DOM surface bound, mapped 1:1 to the merged `crates/dom` public API

Only what `crates/dom` (as merged) actually exposes — no invented methods:

| JS surface | Backed by (`machina_dom`) |
|---|---|
| `Node.prototype.nodeType`/`parentNode`/`firstChild`/`lastChild`/`nextSibling`/`previousSibling` | `Document::node(handle) -> NodeRef` accessors |
| `Node.prototype.appendChild`/`insertBefore`/`removeChild`/`replaceChild` | `Document::append_child`/`insert_before`/`remove_child`/`replace_child` |
| `Node.prototype.cloneNode(deep)` | `Document::clone_node` |
| `Element.prototype.tagName` | `Document::tag_name` |
| `Element.prototype.getAttribute`/`setAttribute`/`removeAttribute` | `Document::attribute`/`set_attribute`/`remove_attribute` |
| `Element.prototype.namespaceURI` (internal use, not necessarily spec-string-exposed yet) | `Document::element_namespace` |
| `CharacterData`-shaped `.data` on `Text`/`Comment` | `Document::text_data`/`set_text_data` |
| `Node.prototype.childNodes` (live-ish `NodeList`) | `Document::children` — see 1.4, this is NOT a live spec `NodeList` in T07 |
| document-level: `document.createElement`/`createTextNode`/`createComment`/`createDocumentFragment` | `Document::create_element`/`create_text`/`create_comment`/`create_document_fragment` |

Explicitly **not** bound in T07 (no backing API exists yet, or out of `crates/dom` scope per
its own non-goals): `innerHTML`/`outerHTML` (dom crate has no serializer), `querySelector*`
(→ M2-T10), shadow DOM/custom elements (→ M3-T04), any event surface (→ M2-T08/T11),
`adoptNode`/cross-document ops exposed at the JS/`document` level (the Rust
`Document::adopt_node` exists, but wiring "move a node between two live `Document`s from JS"
needs a document-registry concept that doesn't exist yet — flagged as OQ-4, not silently
dropped).

### 1.3 Wrapper object shape

Each live JS wrapper is a plain V8 `Object` created from a per-`NodeKind` `ObjectTemplate`
(itself created from a `FunctionTemplate` baked in the snapshot) with:

- **One internal field** (`SetInternalFieldCount(1)`), holding a **native pointer to a
  heap-allocated `WrapperSlot`** (not the `NodeHandle` bytes directly — see 3.2 for why),
  set via `SetAlignedPointerInInternalField`.
- No JS-visible own properties beyond what's defined on the prototype (matches real DOM
  wrapper shape: instance objects are almost empty, behavior lives on the prototype).
- A `v8::WeakCallbackInfo` finalizer registered when the wrapper is created, freeing the
  `WrapperSlot` when V8's GC collects the wrapper (3.3).

`document` itself is a wrapper exactly like any other `Node`-kind object (its `NodeHandle` is
`Document::root()`), not a special-cased object — keeps one code path for the whole surface.

### 1.4 `childNodes` / live collections — deliberately NOT live in T07

`Document::children()` returns an owned `Vec<NodeHandle>` snapshot at call time (per the merged
`crates/dom` API — there is no live cursor type). Binding this directly to a JS `NodeList` that
is supposed to auto-update as the tree mutates would require either (a) polling `Revision` on
every access (cheap, and is exactly the design's intended use of `Revision` per the T05 doc's
"future query-cache layer" note) or (b) a true live view backed by `WrapperObserver`
`Inserted`/`Detached` events. **T07 does (a) only**: `childNodes` returns a fresh
array-like snapshot on every access, tagged internally with the `Revision` it was built from,
diffed lazily against `document.revision()` if re-read — not a spec-exact live `NodeList`.
Building a truly live, spec-shaped `NodeList`/`HTMLCollection` is deferred (flagged OQ-5); doing
it now would front-run M2-T10's selector/live-query work and this task's own "binding scaffold"
framing.

## 2. Snapshot strategy — what's baked vs built per-context

**Baked into the snapshot (`SnapshotCreator`, one-time, offline):**
- `FunctionTemplate`/`ObjectTemplate` definitions for `Node`, `Element`, `Text`, `Comment`,
  `DocumentFragment`, `DocumentType`, `Document`, `Window` — including their prototype chains,
  named/indexed property handler configuration (for future `NodeList`-shaped things), and
  accessor/method callback function pointers (V8 external references, resolved by index —
  V8 requires every native function pointer referenced by a snapshot to be re-supplied via an
  `ExternalReferenceTable` at load time, since raw code pointers cannot be serialized).
- The global object template shape (`window`'s own layout, `document` accessor slot).
- Immutable, page-independent built-ins layered on top of V8's own defaults (per
  `V8_INTEGRATION.md`'s "immutable built-ins safe to share").

**Built per-context, at `Context::create` / document-bind time (not in the snapshot):**
- The actual wrapper *instances* — never baked; instances always hold a per-document
  `WrapperSlot` pointer, and the snapshot has no notion of a document.
- The `document` global's live binding to a specific `machina_dom::Document` — set once when a
  context is associated with a page/document (a new bridge call, `machina_v8_context_bind_document`
  or similar — see OQ-2, this doesn't exist in T06's ABI yet).
- The `WrapperCache` (3.4) — one per `Context`, empty at context creation, populated lazily as
  JS code actually touches nodes.

**Determinism / integrity, per `V8_INTEGRATION.md` ("CI verifies that a runtime cannot load a
mismatched snapshot"):**
- Snapshot build is a fixed, scripted step (deterministic template registration order —
  V8 snapshots are sensitive to *creation order*, not just content) run in CI, producing a blob
  + a content hash.
- The hash, plus the exact V8 revision (from `toolchains/versions.toml`) and the bridge ABI
  version (from T06's `machina_v8_bridge_abi_version()`), are embedded as a header in the
  snapshot file and re-checked at every `machina_v8_platform_init`-adjacent snapshot-load call.
  Any mismatch (wrong V8 build, wrong bridge ABI, corrupted/truncated blob, wrong external
  reference table size) is a **typed rejected-load**, not a crash and not silent fallback to
  cold init — this is the literal "mismatched snapshot is rejected explicitly" acceptance
  criterion.
- **Fallback initialization path**: if no snapshot is present/loadable (e.g., first local dev
  build before the snapshot pipeline has run, or an explicit `--no-snapshot` test mode), the
  bridge builds the same template graph programmatically at `Isolate`/`Context` creation
  instead of deserializing it. Same Rust-side template-registration code backs both paths (the
  snapshot step just serializes the result of running it once) — so there is exactly one source
  of truth for "what the global scaffolding looks like," not two implementations that can drift.
  This satisfies "verify snapshot/build flags and fallback initialization behavior" without
  requiring two independently-maintained binding definitions.

## 3. Handle lifetime: `machina_dom` generational handles vs V8 GC — the hard part

### 3.1 The mismatch, stated precisely

- `NodeHandle` is `Copy`, has no destructor, and is safe to hold indefinitely as *inert data* —
  dereferencing a stale one just returns `DomError::StaleHandle`. It carries no notion of "is
  anyone still using this."
- A V8 wrapper object is reachable for as long as JS code (or another JS object) references it,
  and V8's GC decides when to collect it — asynchronously, not deterministically, and not
  necessarily ever if the isolate itself never runs a full GC cycle.
- These two lifetimes are **not the same** and must not be conflated:
  - A node can be freed on the Rust side (`destroy_node`, or `Document::close()`) while JS still
    holds a live wrapper referencing its `NodeHandle` — every subsequent access must resolve to
    a typed "detached/stale" JS-visible error (`V8_INTEGRATION.md`: "Native node destruction
    invalidates handles; JavaScript access receives a detached/stale-object error").
  - A wrapper can go unreachable and get GC'd while the underlying node is still very much alive
    in the DOM tree (e.g., script drops its only reference to an `Element` still attached to the
    document) — this must **not** free or otherwise touch the DOM node; it only means "if JS
    asks for this node again (e.g., via `parentNode.childNodes[0]`), a *new* wrapper object gets
    created," which is legal (DOM wrapper identity is not spec-guaranteed to be a strict pointer
    match across every access path in a from-scratch engine, though same-wrapper identity for
    repeated access to the *same* live node is a desirable, testable property — see 3.4).

### 3.2 `WrapperSlot` — the owned indirection that bridges the two worlds

Rather than storing a `NodeHandle`'s raw bytes directly in the V8 object's internal field
(tempting, since it's `Copy` and small — 24 bytes per the merged `handle.rs` — but wrong: V8
internal fields are one aligned pointer, not an arbitrary byte payload, and there is no honest
place to write "this node is now gone" if the bytes are just copied), each wrapper's internal
field points to a **heap-allocated `WrapperSlot`** owned jointly by the wrapper (via the
pointer) and the per-context `WrapperCache`:

```rust
// crates/runtime-v8 (new module, name TBD — see OQ-1), NOT in crates/dom.
struct WrapperSlot {
    handle: NodeHandle,           // Copy, cheap
    document: DocumentId,         // redundant w/ handle.document(), kept for fast cache eviction
    state: Cell<WrapperState>,    // Live | Detached | Freed
}
enum WrapperState { Live, Detached, Freed }
```

- **On DOM mutation/teardown** (`WrapperObserver` callbacks, delivered synchronously per T05's
  contract — "notified after the fact", not async): `on_node_changed(handle, Detached)` sets the
  matching slot's `state = Detached` (still resolvable via `Document::node`, per T05's own
  distinction); `on_node_freed(handle)` sets `state = Freed` and evicts the cache entry;
  `on_document_teardown(document)` walks the per-context `WrapperCache` for that document and
  sets every slot to `Freed` in one pass (matches T05: "fires once, not per-node" — the cache
  scan is this binding layer's job, not the DOM crate's).
- **On V8 GC of the wrapper**: the `v8::WeakCallbackInfo` finalizer frees the `WrapperSlot`
  (`Box::from_raw` + drop) and removes it from the `WrapperCache`. It does **not** touch
  `machina_dom` at all — a wrapper going away never frees or mutates a DOM node. This is the
  precise asymmetry that resolves 3.1: DOM→wrapper notifications are synchronous state
  transitions on an already-allocated slot; wrapper→DOM has no analogous callback because there
  is nothing for the DOM crate to be told (it doesn't track wrapper existence at all — that's
  the whole point of `WrapperObserver` being a single, minimal, DOM-crate-owned hook rather than
  a two-way relationship).
- **Every accessor/method callback re-checks `state`** before calling into `machina_dom`:
  `Freed` → throw a `DOMException`-shaped JS error (`InvalidStateError`-equivalent, mapped per
  §4); `Detached` → still allowed for read/structural ops that are valid on a detached node
  (matches `Document::node` itself still succeeding on a merely-detached handle), consistent
  with T05's explicit "detached-but-resolvable" semantics. This makes the check trivial and
  branch-cheap (`Cell<WrapperState>` read, no re-resolution needed to detect "definitely gone")
  while still being **defense-in-depth, not the only check** — the callback still calls the real
  `Document` method, which independently re-validates via `resolve()`/generation check. A
  `WrapperSlot` marked `Freed` is a fast-path denial; a `WrapperSlot` that's stale despite saying
  `Live` (a bug) still fails safely at the `machina_dom` layer, never UB.

### 3.3 Weak callback registration — two-phase GC, not immediate free

V8 weak callbacks run in two phases (`SetWeak` with `kParameter` vs `kInternalFields` type, and
first-pass vs second-pass callbacks) specifically because a first-pass callback runs *before*
the object is actually reclaimed and must not allocate/run arbitrary V8 API calls; only a
second-pass callback (or a call scheduled via `SetSecondPassCallback`) may safely trigger
further V8 operations. This binding layer's finalizer:
- **First pass**: mark-only — read the raw pointer out of the internal field, schedule a
  second-pass callback carrying just that pointer (no other V8 calls).
- **Second pass**: `Box::from_raw(ptr)` to reclaim the `WrapperSlot`, remove it from the
  `WrapperCache` (a plain Rust `HashMap`, no V8 call needed for that step).

This detail is spelled out because it's the single easiest thing to get subtly wrong (a
first-pass callback that touches the `WrapperCache`'s owning `Context` state directly, or that
allocates a new V8 handle, is a documented source of use-after-free/crashes in other engines'
embedder code) and is exactly the kind of invariant `AGENTS.md`'s "no `unsafe` without a
documented invariant + focused test" rule expects to be called out explicitly, plus it needs a
dedicated GC-cycle stress test (§6).

### 3.4 `WrapperCache` — identity stability, not just lifetime

`WrapperCache: HashMap<NodeHandle, v8::Weak<v8::Object>>` per `Context`. Every accessor that
would otherwise create a new wrapper for a `NodeHandle` (e.g., `.parentNode`, `.childNodes[i]`)
first checks the cache; a live weak reference found there is upgraded and returned instead of
allocating a new wrapper. This gives `parentNode.firstChild.parentNode === parentNode`-style
identity stability for as long as *something* keeps at least one of the wrapper references
alive — matching ordinary browser behavior — while still allowing GC to reclaim genuinely
unreferenced wrappers and rebuild them later from the cheap `NodeHandle`. Cache eviction: on
`Freed` (see 3.2) and lazily on cache-hit-but-weak-reference-already-cleared (standard weak-map
pattern, no separate GC-driven cache sweep needed).

## 4. Binding-generation approach: hand-written, not macro/codegen — for T07

- **Hand-written `ObjectTemplate`/callback registration**, mirroring T06's own decision
  ("nothing safety-critical is auto-generated; this is the audited part ADR-002 calls for") and
  `crates/dom`'s own convention (no `thiserror`, no derive-heavy macro reliance anywhere in the
  workspace so far).
- A **shared internal helper macro is acceptable and recommended for the repetitive glue only**
  (argument-count/type checking, `WrapperSlot` state-check boilerplate, exception-throwing
  boilerplate) — explicitly *not* a schema/IDL-driven generator. The per-accessor business
  logic (which `Document` method to call, how to map the result) stays hand-written and
  reviewable.
- **Why not a Web IDL-style codegen tool now**: the bound surface is small (single-digit
  interfaces, ~20 members total per §1.2) and `V8_INTEGRATION.md` only says "Web IDL-style
  bindings" descriptively, not "must be IDL-generated." Revisit if/when M3's much larger surface
  (forms, shadow DOM, frames, workers) makes hand-written glue's maintenance cost exceed a
  codegen tool's setup cost — explicitly flagged as a later decision, not foreclosed here, and
  not something T07 should preempt by picking a codegen tool for a 20-member surface.
- Concrete shape: one `fn install_node_template(isolate) -> Local<ObjectTemplate>` per
  `NodeKind`-family interface, each registering its accessors/methods via small
  `v8::FunctionCallback`-shaped trampolines that: (1) unwrap `this`'s internal field →
  `WrapperSlot`, (2) state-check (3.2), (3) unwrap/validate arguments, (4) call the matching
  `machina_dom::Document` method (needs the bound `Document` reachable from the callback — via
  a `Context`-embedder-data slot pointing at a `RefCell<Document>` or equivalent single-owner
  cell, since callbacks don't otherwise carry Rust closures across the C ABI — flagged as
  needing T06 coordination, OQ-3), (5) map `Result<T, DomError>` to either a JS return value or
  a thrown exception (§5), (6) map any produced/returned `NodeHandle` back to a wrapper via the
  `WrapperCache` (3.4).

## 5. Error/exception mapping: Rust `Result` ↔ JS exceptions

Two independent directions, not one shared table, because they cross the boundary differently:

### 5.1 JS calls into a native binding, native call fails (`DomError` → JS exception)

Native callbacks throw synchronously via `isolate->ThrowException(...)`, matching normal
JS semantics (a `Node.prototype.appendChild` call that violates a hierarchy constraint throws,
it does not return a sentinel). Mapping, deliberately DOM-exception-shaped (not
`CanonicalErrorCode`-shaped — `DomError` is an engine-internal-tree-invariant error, not a
command-bus error; conflating the two would leak an implementation detail into page-visible
exception text):

| `DomError` | JS exception (name / message shape) |
|---|---|
| `StaleHandle`, `WrongDocument` (via `WrapperSlot.state == Freed` fast path or the underlying resolve) | `DOMException("InvalidStateError", "node is no longer part of a live document")` |
| `WrongKind` | `TypeError` (calling an element-only accessor on the wrong kind is a programmer error, not a DOM-state error — matches how real engines throw `TypeError` for interface mismatches) |
| `NotFound` | `DOMException("NotFoundError", ...)` |
| `HierarchyViolation` | `DOMException("HierarchyRequestError", ...)` |
| `SameDocument` | `DOMException("NotSupportedError", ...)` (only reachable if/when `adoptNode` is exposed — OQ-4) |
| `DepthLimitExceeded` | `DOMException("HierarchyRequestError", "tree depth limit exceeded")` — deliberately still a spec-shaped exception, not an internal error, since from JS's perspective this is indistinguishable from "your tree shape is invalid" |
| `NodeStillAttached`, `NodeHasChildren` | Not JS-reachable in T07 (`destroy_node` is not called from any exposed binding — GC-driven eviction never calls it, per 3.2/3.3; these variants only matter to whatever later task explicitly exposes node removal-from-arena as opposed to detach) |
| `InvalidName` | `TypeError` |
| `DocumentClosed` | `DOMException("InvalidStateError", "document is no longer active")` |

`DOMException` itself is a small hand-written `ObjectTemplate` baked into the snapshot
(constructor + `name`/`message`/`code` own properties), not V8's built-in `Error` — matches
what real DOM bindings expose and gives page script the ability to `catch (e) { if (e.name ===
'NotFoundError') ... }`, which machine-workload extraction/automation code plausibly depends on.

### 5.2 Native (Rust) code needs to react to a JS-thrown/uncaught exception

This is **T06's existing contract, reused as-is, not re-invented**: `machina_v8_context_execute`
already returns `MachinaV8ExecuteResult{outcome: EXCEPTION, exception_kind, exception_message,
exception_stack, source_location}` via `v8::TryCatch` around `Compile`/`Run` (T06 design §2/§6).
T07 does not add a second exception-capture path for *script-level* exceptions — a binding
throwing (5.1) surfaces through exactly that same `TryCatch` machinery when the throwing call
happens synchronously inside a running script. T07's only addition is *populating* that
existing `exception_kind`/`exception_message` pair correctly when the thrown value is a
`DOMException` object rather than a plain `Error` (i.e., `exception_kind` should read
`"DOMException:InvalidStateError"` or similar structured form, not just `"Error"` — a
concrete, testable requirement worth calling out to the T06 builder since T06's own test list
only exercises plain thrown/syntax errors, per its §6 table).

Per `V8_INTEGRATION.md` ("Page exceptions are events unless the command contract makes them
fatal") and T06's own explicit deferral ("`Exception` does NOT auto-map to `DispatchError`...
deciding how `Exception` becomes an event is deferred"): T07 does not change that deferral. A
thrown `DOMException` from a binding is just another `ExecuteOutcome::Exception` as far as T06's
facade and whatever later task wires this into `native-core` are concerned.

## 6. Test strategy mapped to acceptance criteria

| Criterion | Tests |
|---|---|
| "Snapshot-loaded context exposes expected globals and DOM wrappers" | `snapshot_load_exposes_window_document_globals`; `create_element_from_js_returns_element_wrapper_with_correct_prototype_chain` (`instanceof Node`/`instanceof Element`); `append_child_from_js_mutates_dom_and_is_visible_via_children()` (round-trip through `machina_dom`, not just template shape) |
| "Mismatched snapshot is rejected explicitly" | `snapshot_wrong_v8_revision_hash_rejected`; `snapshot_wrong_bridge_abi_rejected`; `snapshot_truncated_blob_rejected`; `snapshot_missing_falls_back_to_programmatic_init_successfully` (fallback path, §2, must not silently degrade functionality — same wrapper behavior asserted under both paths) |
| "Warm isolate/context startup baseline is recorded" | Benchmark harness (per fast gate: "benchmark cold versus snapshot startup on reference host") recording p50/p99 context-creation wall time with vs without snapshot, written to a tracked baseline file so future regressions are visible in CI, not just eyeballed once |
| Handle-lifetime correctness (3.1-3.4, not a stated acceptance line item verbatim but load-bearing for "DOM wrappers" to mean anything) | `wrapper_survives_gc_while_node_still_live_then_gc_reclaims_after_all_js_refs_dropped` (forced GC via V8 test API); `access_after_native_destroy_node_throws_invalid_state_not_uaf` (run under ASan, reusing T06's sanitizer job); `document_teardown_invalidates_every_live_wrapper_in_one_pass`; `wrapper_identity_stable_across_repeated_property_access_while_referenced` (3.4); `weak_callback_second_pass_reclaims_slot_no_leak_under_repeated_create_gc_cycles` (leak smoke, LSan-checked, targeting the exact two-phase-callback risk in §3.3) |
| Error mapping | One JS-visible test per `DomError` row in §5.1's table, asserting `name`/`message`/prototype chain of the thrown value, not just "it throws" |

Fast gate: snapshot load/hash/reject tests + cold-vs-warm benchmark, per the task's own stated
fast gate. Handle-lifetime/GC-cycle tests belong in the sanitizer job (reuses T06's
`bridge-sanitizer` CI job rather than standing up a second one) since they're exactly the class
of test ASan/LSan is for.

## 7. Module layout (indicative — final crate/module split is OQ-1)

```
crates/runtime-v8/src/
  bindings/                  # new, added by T07 on top of T06's existing layout
    mod.rs
    snapshot.rs              # SnapshotCreator invocation, hash/version header, load+verify
    templates.rs              # install_node_template / install_element_template / ... (§4)
    wrapper.rs                 # WrapperSlot, WrapperCache, weak-callback two-phase glue (§3)
    exceptions.rs                # DomError -> DOMException mapping (§5.1), DOMException template
    document_binding.rs           # per-Context Document binding slot, embedder-data plumbing (OQ-3)
  tests/
    snapshot_integrity.rs
    wrapper_lifetime.rs           # GC-cycle / handle-lifetime tests, run under sanitizer job
    dom_binding_surface.rs        # §1.2 table, one test per bound member
    exception_mapping.rs
cpp/v8-bridge/
  include/machina_v8_bridge.h     # extended: snapshot create/load/verify entry points (new to T07)
  src/snapshot.cpp                 # new: SnapshotCreator driver, external-reference-table wiring
scripts/build/v8-snapshot/          # new: deterministic snapshot-build script + CI integration
```

## 8. Open questions / coordination needed with the M2-T06 builder

- **OQ-1 (crate placement).** This doc assumes the binding layer lives inside `crates/runtime-v8`
  (new `bindings` module) rather than a separate `crates/dom-bindings` crate. Reasoning: it
  needs deep access to V8 template/handle machinery that T06's facade is written to keep
  private (`sys.rs` is explicitly private per T06 §5), so a separate crate would either need
  T06 to expose more raw V8 surface (undesirable — re-widens exactly the boundary ADR-002 narrows)
  or would need its own C ABI additions duplicating T06's. **Needs T06 builder agreement**: is
  `crates/runtime-v8` willing to grow a `bindings` module owned by a different task/PR, or does
  it want a narrower export surface with binding logic elsewhere?
- **OQ-2 (context-to-document binding entry point).** T07 needs a new bridge/facade call to
  associate a live `machina_dom::Document` with a `Context` (so `document` resolves to
  something, and so the bound `Document` is reachable from callbacks — see OQ-3). Nothing in
  T06's current ABI (§2 of its design doc) does this — its `Context` is purely a V8 execution
  context, document-agnostic. **Needs T06 builder input**: should this be a new bridge ABI
  function (`machina_v8_context_bind_document`) or purely a Rust-facade-level API added on top
  of T06's existing `Context<'iso>`? Either is workable, but it changes whether T07 needs new
  C++ or can stay Rust-only.
- **OQ-3 (reaching `Document` state from a native callback).** V8 `FunctionCallback` trampolines
  are bare `extern "C"` function pointers with no captured closure state; the only place to
  stash "which `Document` does this context's wrappers resolve against" is V8's per-`Context`
  or per-`Isolate` embedder-data slot (`SetEmbedderData`/`GetAlignedPointerFromEmbedderData`).
  T06's design doc doesn't mention embedder-data slots at all. **Needs T06 builder input**:
  confirm no other T06 subsystem is already claiming embedder-data slot index 0 (or agree on a
  slot-index allocation scheme), so T07 and T06 (and later T08's lane/microtask state, which
  also needs per-context reachable state per its own design doc) don't collide.
- **OQ-4 (cross-document operations from JS).** `Document::adopt_node` exists in `crates/dom`,
  but nothing in this design exposes a JS-visible way to move a node between two different
  `Document`s (there's no "other document" object model yet — multiple documents/frames are
  M3-T05 scope). Flagged as explicitly deferred, not silently dropped: `SameDocument`'s mapping
  in §5.1 is listed as "only reachable if/when `adoptNode` is exposed."
- **OQ-5 (live `NodeList`/`HTMLCollection`).** §1.4 intentionally ships a non-live snapshot
  array for `childNodes`, deferring true live collections. This is a legitimate design
  simplification for T07 but worth flagging to whoever picks up M2-T10 (selectors) or any WPT
  compatibility pass, since "live NodeList" is one of the more commonly-tested DOM behaviors.
- **OQ-6 (per-callback native call-stack depth guard).** T06's security review (§4) calls for
  "native call-stack depth bounded independently ... protects against getter→native→script→getter
  recursion." T07's binding trampolines are exactly where that recursion pattern would occur (a
  JS getter defined via `Object.defineProperty` on a DOM wrapper's prototype, or a `Proxy`,
  calling back into a native accessor which itself could re-enter JS via a future
  accessor-with-side-effects). T07 should implement the counter T06's review calls for, but the
  counter needs to be visible/shared with T06's own facade if T06 ends up implementing part of
  it first — needs explicit coordination on which task owns this counter's storage (likely: T06
  owns the mechanism/threshold since it's isolate-scoped safety infrastructure; T07's
  trampolines just call into it).

## Files reviewed

`planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md` (M2-T07 + neighboring M2-T05/T06/T08/T09) ·
`agents/TASK_PACKET_TEMPLATE.md` · `AGENTS.md` · `architecture/ADR/ADR-001-HYBRID_ENGINE.md` ·
`architecture/ADR/ADR-002-RUST_V8.md` · `architecture/V8_INTEGRATION.md` ·
`architecture/NATIVE_ENGINE.md` · `.agent-state/design/M2-T06-v8-bridge-design.md` ·
`.agent-state/design/M2-T06-security-review.md` · `.agent-state/design/M2-T06-toolchain-feasibility.md` ·
`.agent-state/design/M2-T05-dom-design.md` · `.agent-state/design/M2-T08-event-loop-design.md`
(cross-checked for a second, independent set of "what does runtime-v8 need beyond T06's own
task packet" findings) · `crates/dom/src/{lib,handle,document,node,mutation,observer,error}.rs`
(merged, read directly) · `crates/command-model/src/generated.rs` (`CanonicalErrorCode`, to
confirm §5.1 deliberately does NOT reuse it).
