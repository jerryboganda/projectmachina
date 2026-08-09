# Design: M2-T05 — Compact DOM Nodes, Handles, Mutation and Lifecycle

> Produced by a wave-1 architect research agent ahead of M2-T05 implementation.
> Read-only design; no code changes. Feed this directly into the M2-T05 builder prompt.

Scope source: `planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md` (`## M2-T05`). Target crate: `crates/dom` (currently empty, **not yet a workspace member** — same pattern found in `crates/network`/`crates/security-policy` by the wave-1 security reviews). Conventions preserved from existing crates (`session`, `native-core`, `capability`, `command-bus`): no `unsafe`, no `unwrap`/panic on caller-reachable paths, hand-written `Display`/`Error` enums (no `thiserror` anywhere in the workspace), minimal dependency graphs (path-deps only), small `Copy` newtype IDs. `crates/dom` must have **zero** dependency on `command-model`, `command-bus`, `session`, or protocol crates.

## 0. Design summary

One `Document` = one arena = one generation space = one teardown unit. Handles are `Copy` 20-byte values `(DocumentId, NodeIndex, Generation)`. Every dereference is O(1) checked against the arena; a stale or wrong-document handle can never alias live memory or panic — returns a typed `DomError`. Exactly one mutation-notification hook (`WrapperObserver`, single-slot, minimal) so the V8-bridge design (M2-T06/T07) can build whatever multiplexing it needs on top, without this crate guessing at V8 concerns.

## 1. Storage model

- **Arena, not `Rc<RefCell<Node>>`.** Each `Document` owns one `NodeArena { slots: Vec<Slot>, free_list: Vec<NodeIndex>, live_count }`, `Slot { generation: u32, data: Option<NodeData> }`. One-arena-per-document makes cross-document handle detection O(1) via `DocumentId` comparison and makes teardown structural (dropping `Document` drops the one `Vec<Slot>`, no `Rc`/`Arc` node owners anywhere → reference cycles impossible by construction).
- Tree edges are handle fields on each node (`parent`, `first_child`, `last_child`, `next_sibling`, `previous_sibling` — doubly linked sibling list), not a `Vec` of children per node — O(1) insert/remove, zero per-node allocation beyond the arena's own `Vec`.
- **Generational handles:** `DocumentId(u64)` (process-unique, `AtomicU64`), `NodeIndex(u32)`, `Generation(u32)`, `NodeHandle { document, index, generation }`. `Document::resolve()` is the single choke point: checks wrong-document → document-closed → stale-generation → freed-slot, always via `Vec::get` (never `[]`), so a forged/corrupted handle returns `DomError::StaleHandle` instead of panicking.
- Kind-typed handles (`ElementHandle`, `TextHandle`, `DocumentFragmentHandle`) wrap `NodeHandle`, produced only via a kind-checked `Document::as_element()`-style downcast.
- **Interning, two-tier:** static compiled-in table (well-known tag/attribute names, `OnceLock`, zero-cost, process-lifetime, holds no page content) + a dynamic per-`Document` `StringInterner` for custom tags/attributes (dropped with the document, bounding adversarial custom-tag growth to that document's own accounted memory). Attribute values and text data are `Box<str>`, not interned (arbitrary, largely-unique). `AttributeMap` is `Vec<(Atom, Box<str>)>` (linear scan, insertion order preserved, matches `NamedNodeMap` semantics) — not `HashMap`.

## 2. Core API surface

`NodeKind`: `Document, DocumentType, Element, Text, Comment, DocumentFragment`.

Key methods on `Document`: `create_element/create_text/create_comment/create_document_fragment`, `node()`/`as_element()` (read), `insert_before/append_child/remove_child/replace_child/adopt_node/clone_node` (mutation), `set_attribute/remove_attribute/attribute/set_text_data`, `revision()`, `memory_usage()`, `set_wrapper_observer()`, `close()`.

**Tree-invariant guarantees:** every mutating method is two-phase (validate everything, then commit) — no partially-relinked tree is ever visible after an `Err`. Enforced invariants, each with an explicit `DomError` variant: same-document only (`WrongDocument`); inserting an already-attached node implicitly detaches it from its old parent first (never duplicates into two trees); no cycles (`HierarchyViolation`, ancestor walk bounded by a documented `MAX_ANCESTOR_WALK` constant so adversarially deep trees fail closed); `reference`/`child` must currently belong to `parent` (`NotFound`); `Document`-kind parent accepts at most one `Element` child and one `DocumentType` child.

**Explicit decision — no implicit cross-document adoption.** Living DOM auto-adopts on `insertBefore`; this design rejects cross-document inserts and requires an explicit `adopt_node` call instead, to keep every mutation auditable through one code path and keep wrapper-invalidation simple (no hidden second mutation inside insert). A thin binding-layer wrapper can compose `adopt_node` + `insert_before` later if spec-exact atomic auto-adopt is needed for WPT compatibility — flagged as a documented decision to revisit only if JS-binding WPT work later requires it.

`adopt_node`: detaches subtree from source's arena (freeing those slots — turning other handles into that subtree from elsewhere into stale handles, which is the desired "gone from old document" semantic), re-creates equivalent slots in destination, re-interns dynamic-tier atoms. `source == self` → `DomError::SameDocument`.

`clone_node`: always same-document, fresh slots/handles, deep clone iterative (explicit stack, not recursion) bounded by the same depth guard as cycle checks — same fuzz-hardening posture as the M2-T03 tokenizer.

## 3. Mutation revisions and wrapper-invalidation hooks

- `Revision(u64)` — one monotonic counter per `Document`, bumped on every successful mutation (structural or content). Deliberately coarse-grained (not split structural-vs-content) since M2-T10 only needs "did anything change since last cache" — finer split is a documented deferred optimization, not a gap. Each `NodeData` also stamps `modified: Revision` for free, for later per-node semantic-diff work.
- **`WrapperObserver` trait — deliberately minimal, zero V8 types in its signature:**
  ```
  trait WrapperObserver: Send {
      fn on_node_changed(&self, handle: NodeHandle, change: NodeChange); // Inserted/Detached/AttributesChanged/TextChanged
      fn on_node_freed(&self, handle: NodeHandle);      // slot freed, handle now permanently stale
      fn on_document_teardown(&self, document: DocumentId); // fires once, not per-node
  }
  ```
  Single observer slot (not `Vec<Box<dyn ...>>`) — fan-out, if ever needed, composes on the caller's side. `on_node_changed`/`on_node_freed` are deliberately separate (a node can be `Detached` — still resolvable — long before being freed; conflating would force the V8 bridge to guess). No return value/error type — the observer cannot fail a mutation, it's notified after the fact. Threading contract: `Document` usable from one thread/task only (mirrors V8 isolate confinement); `Send` required, `Sync` not needed since `crates/dom` never calls it concurrently.

## 4. Document teardown

- `DomMemoryUsage { node_count: u64, bytes_estimate: u64 }`, tracked incrementally, exposed via `Document::memory_usage()`. `crates/dom` only *reports* usage — it never depends on `machina_session`; the owning layer (native-core, M2-T01) is responsible for reconciling against `ResourceBudget`/`ResourceUsage` and refusing further mutation at a higher level.
- **Explicit `Document::close()`** (idempotent): notify `on_document_teardown` once → replace arena with an empty one (drops every `Slot`, releasing all owned `String`/`Box<str>` synchronously, not on a GC pass) → zero counters → set `torn_down`. `impl Drop for Document` calls `close()` unconditionally, so scope-drop without explicit close still tears down correctly and still fires the notification exactly once.
- No lingering references **by construction, not discipline**: zero `Rc`/`Arc`/raw pointers for tree structure, so a stale `NodeHandle` is inert data (two integers + a document id) — holding one after teardown cannot touch freed memory, only fail a bounds/generation check. This is the direct, testable mechanism for "teardown releases accounted memory without lingering references."

## 5. Concrete crate module layout

```
crates/dom/
  Cargo.toml          # no dependencies beyond std — keep as a leaf crate so
                       # M2-T04/T06/T07/T10 can all depend on it without cycle risk
  src/
    lib.rs             # re-exports
    handle.rs          # DocumentId, NodeIndex, Generation, NodeHandle + kind-typed handles
    arena.rs           # Slot, NodeArena (pub(crate)): alloc/free/resolve, free-list reuse
    node.rs            # NodeKind, NodeData, NodeLinks, NodeRef<'a> borrowed read view
    document.rs        # Document struct, constructors, attribute/text methods, revision,
                        # memory_usage, set_wrapper_observer, close, Drop impl
    mutation.rs         # insert_before/append_child/remove_child/replace_child/adopt_node/
                        # clone_node + two-phase validate-then-commit checks
    intern.rs            # Atom, StringInterner (static OnceLock table + per-document dynamic)
    attributes.rs         # AttributeMap (Vec<(Atom, Box<str>)>, ordered, linear scan)
    observer.rs            # WrapperObserver trait, NodeChange (no V8 types)
    error.rs                # DomError enum + Display + std::error::Error (hand-written)
  tests/
    handles.rs               # stale-handle / cross-document-handle failure tests
    mutation_invariants.rs    # insert/remove/replace/adopt/clone invariant tests
    teardown.rs                # bulk teardown / repeated create-destroy memory tests
    observer.rs                 # wrapper-notification ordering/content tests
```

## 6. Test strategy mapped to acceptance criteria

**`tests/handles.rs`:** stale handle after free (`StaleHandle`, not panic) across every op · index/generation-collision-across-documents still rejects via `WrongDocument` · handle from doc A used in doc B's mutation methods → `WrongDocument`, B's tree unchanged (snapshot diff) · generation-reuse: old handle to a reused index still fails, new handle succeeds · post-`close()` any handle → `DocumentClosed`.

**`tests/mutation_invariants.rs`:** re-parenting detaches from old parent exactly once (no duplication) · cycle rejection leaves tree unchanged (`HierarchyViolation`) · bad `reference`/`child` → `NotFound`, unchanged · `replace_child` preserves sibling position, old handle stays valid-but-detached · `adopt_node` stales the old handle, preserves subtree structure/content deep-equal in the destination, re-interns dynamic atoms correctly · `adopt_node(self, self)` → `SameDocument` · shallow vs deep clone correctness · deep/wide clone at the depth-guard boundary fails bounded (no stack overflow) · `Document`-kind parent rejects 2nd `Element`/any `Text` child · `revision()` strictly increases on success, unchanged on `Err` · a small fixed-seed pseudo-random mutation sequence (hand-rolled LCG — no `proptest`/`rand` precedent exists in this workspace yet; flagged as a low-risk optional dev-dependency addition if richer shrinking is wanted) checked against a plain `Vec`-based reference-tree model.

**`tests/teardown.rs`:** `close()` → `memory_usage()` reports zero · every handle issued pre-close fails `DocumentClosed` post-close, uniformly · repeated create/destroy loop (M documents × N nodes) shows no cross-document accumulation and exactly M `on_document_teardown` calls · drop-without-explicit-close still fires teardown exactly once via `Drop`, and `close()` called twice on a still-live value is idempotent.

**`tests/observer.rs`:** `on_node_changed` fires with the correct `NodeChange` variant on each mutation kind, never on a rejected mutation · `on_node_freed` fires exactly once per node, only on actual slot-free (not mere detach) · bulk teardown fires one `on_document_teardown`, not N `on_node_freed` calls (explicit regression test — easy to accidentally implement as a loop) · reentrant `&self` read from inside a callback compiles/behaves correctly (no interior mutability/locks in `Document`).

## 7. Explicit non-goals for M2-T05 (deferred)

HTML parsing/tree construction (→ M2-T04, which calls into this API) · CSS selectors/XPath/live query indexes (→ M2-T10, this crate only guarantees `Revision` to key cache invalidation on) · Shadow DOM/slots/composed-tree/custom-element registry (→ M3-T04; `NodeKind` left as an open enum so `ShadowRoot` can be added later without restructuring the arena, nothing shadow-specific implemented now) · V8 wrapper objects/isolate/GC integration (→ M2-T06/T07; `WrapperObserver` is the entire surface exposed toward that work) · semantic CSS/visibility/geometry/hit-testing (→ M3-T12) · frames/browsing contexts (→ M3-T05) · memory-budget *enforcement* (that's `crates/session`'s job — this crate only *reports* via `memory_usage()`) · serialization (innerHTML/outerHTML) — not in M2-T05's deliverables, left for a later task · full foreign-content namespace semantics (→ M2-T04's "foreign content" deliverable; data model leaves room for a namespace `Atom` field but doesn't build it out).

## Files reviewed

`planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md` (M2-T05 + neighboring M2-T04/T06/T07/T10) · `planning/MASTER_TASK_GRAPH.md` · `planning/MILESTONE_03_NATIVE_WEB_APIS_AND_AUTOMATION.md` (M3-T04 boundary check) · `AGENTS.md` · `research/TECHNOLOGY_SELECTION.md` · `research/LIGHTPANDA_GAP_ANALYSIS.md` · `crates/session/src/lib.rs` · `crates/native-core/src/lib.rs` · `crates/capability/src/lib.rs` · workspace `Cargo.toml` (confirmed `crates/dom` not yet a workspace member; confirmed no property-testing crate used anywhere in the repo today).
