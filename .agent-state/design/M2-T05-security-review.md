# Adversarial Design Review: M2-T05 DOM (Arena / Generational Handles)

> Produced by a wave-2 security research agent, reviewing the M2-T05 design (not code — reviewed
> before/during implementation). Read-only; no files changed. Relayed live to the M2-T05 builder.

**Overall assessment:** core architectural shape (arena, `Copy` generational handles, structural teardown, two-phase mutation) correctly eliminates classic UAF/dangling-pointer/reference-cycle bugs by construction. The real risk is in what's **left unspecified** — node-reclamation trigger, exact per-method commit choreography, concrete bound values.

## Finding 1 — Generation wraparound: real gap, silent-aliasing failure mode

`u32` wraparound on a single hot slot is reachable at realistic churn (~10k create/free/sec on one slot → 2^32 wraps in ~120h — plausible for multi-day automation soak runs), and post-wraparound the failure mode is **silent handle confusion** (an old stale handle whose generation happens to equal current-mod-2^32 passes the check and resolves to an unrelated node), not a safe error. Only one generation-bump is tested; wraparound boundary isn't. **Fix:** widen `Generation` to `u64` (handle grows 20→24 bytes, negligible) — the standard fix (`slotmap`/`generational-arena` precedent) — or add a checked-add-with-permanent-slot-retirement policy on overflow. Add a test that seeds a slot near `u32::MAX` to prove the fix fires. **Interacts with Finding 4**: if Finding 4 adds a real per-node reclaim path, ordinary `removeChild`-heavy workloads make slot reuse much more frequent, making this more urgent.

## Finding 2 — `adopt_node`: mostly sound, two real gaps

Direct reuse-after-adopt correctly fails closed (descendant handles go stale too). Real gaps: (2a) test list only names root-handle staleness, not descendant-handle staleness explicitly — add that test. (2b) **the free+recreate strategy cannot preserve JS object identity across `adoptNode`** — spec requires `document.adoptNode(n)` returns the *same* node object (`n === returnedNode`); this design allocates a new handle, so the V8 bridge would either break `===` identity (WPT/spec regression) or needs a correlation signal the `WrapperObserver` contract doesn't currently provide. Flag explicitly for M2-T06/T07 rather than leaving implicit — either accept/document the spec deviation now, or add `NodeChange::Adopted{old_handle}` so the observer can re-key an existing wrapper.

## Finding 3 — Two-phase commit: three real edge cases

**3a. Self-aliased arguments** (`replace_child(p,x,x)`, `insert_before(p,x,x)`): if commit-phase steps act on validate-phase-*cached* positions rather than live-re-read links, mutating via one step (e.g. detach) can corrupt the cached fact the next step relies on — a genuinely broken tree produced *silently* (the call succeeds). Not tested anywhere. **Fix:** mandate commit-phase steps always re-resolve live links at the start of each micro-step; add explicit self-aliasing tests.

**3b. `clone_node` depth-guard timing ambiguous** — if the depth check is interleaved with destination allocation rather than a read-only pre-pass over the source, tripping the guard mid-walk leaves already-allocated destination slots orphaned (not linked into any tree, no handle returned, but permanently counted in `memory_usage()`) — a real, silent slot leak. **Fix:** mandate a read-only depth-count pre-pass before any destination allocation; test that an over-limit clone leaves `memory_usage()` unchanged.

**3c. `replace_child` vs. "Document accepts ≤1 Element child"**: replacing a Document's existing sole Element child with a *different* Element is a legitimate no-net-change operation. If the invariant check naively pre-counts existing children without excluding `old_child`, this legitimate replace is incorrectly rejected. **Fix:** explicit test for this exact case, asserting success.

## Finding 4 — HEADLINE: no reclaim path for detached-and-abandoned nodes

Walking the entire API surface, **no method frees an individual node's slot for the ordinary single-document case** — only `adopt_node` (frees the source doc's slots as a side effect of a cross-document move) and `close()` (frees the whole arena). A node that's `remove_child`'d and then abandoned (the single most common real-world DOM pattern: SPA re-renders, toasts, polling widgets) has **no path back to reclamation** — no reference counting exists (deliberately, `NodeHandle` is `Copy`), so the arena has no intrinsic way to know a detached node became unreachable, and no explicit call exists to tell it. Consequences: **unbounded memory growth for any long-running Document** under the most common mutation pattern, directly undercutting `memory_usage()`'s own accounting story and eventually tripping the higher-layer `ResourceBudget` for workloads that shouldn't be near it. **`WrapperObserver::on_node_freed` never fires for this case** — the V8 bridge has no signal to drive wrapper-cache eviction for ordinary garbage, only V8's own GC finalizer, which has nothing to call in `crates/dom` to complete release since no destroy API exists. This is DoS/resource-exhaustion-shaped (not memory-unsafe — still bounds/generation-checked, no UB) but a real, consequential gap.

**Fix:** add an explicit reclaim entry point, e.g. `Document::destroy_node(handle) -> Result<(), DomError>` (valid only on a currently-detached node with no live children), that the V8 bridge calls from its wrapper finalizer once a wrapper is unreachable and its node is detached — the missing half of `WrapperObserver`'s lifecycle story. Document as a required M2-T06/T07 integration point. **If** the intended design is instead "detached subtrees retained until document close, full stop," that must be an explicit *named* limitation (not silently absent) so `crates/session`'s resource-budget accounting is built with that assumption in mind, not discovered empirically.

**Secondary test gap (same class as an already-tested case):** `adopt_node` freeing a whole subtree should fire one `on_node_freed` per freed slot, not a batched event — no test named for this, unlike the already-tested `close()` single-teardown-event case. Add it.

**Forward note for V8-bridge review:** whatever wrapper cache M2-T06/T07 builds must key by the full `(NodeIndex, Generation, DocumentId)` tuple, never `NodeIndex` alone — an index-only cache would silently hand out a stale wrapper across slot reuse (adopt or, per Finding 1, wraparound).

## Finding 5 — Miscellaneous

**5a. Real gap:** `MAX_ANCESTOR_WALK` conflates two purposes (fuzz-hardening bound vs. the actual cycle-detection walk) with no stated value — if the same small cap fail-closes the cycle check itself, legitimate-but-deep real pages (page builders, minified frameworks, five-figure depths not unheard of) would have ordinary non-cyclic inserts incorrectly rejected. Since `clone_node`'s walk is already iterative (no stack-safety need for a small bound), **fix**: pick and document a generous value, and/or separate the pathological-depth-DoS budget from the correctness-critical cycle-detection walk.
**5b. Minor:** Document child relative-ordering (DocumentType before Element) isn't enforced by the invariant as stated — low risk since M2-T04's parser will always insert correctly, worth a one-line design note since `dom` is meant as a general API surface.
**5c. Advisory only:** verify at implementation time that "reference/child belongs to parent" is O(1) (uses the stored `parent` field), not an accidental O(children) sibling-scan — would create latent O(n²) DoS on bulk-removal over wide trees. Nothing in the design suggests this is intended; flagging to check once code lands.
**5d. False alarms (checked, not issues):** `Revision`/`DocumentId` u64 wraparound (astronomically unreachable); generation-0/never-allocated-slot aliasing (`Slot.data: Option<NodeData>` already closes this); observer read-reentrancy (already tested; write-reentrancy risk would only appear if an *integration* layer wraps `Document` in `Rc<RefCell<_>>` — a caution for the V8-bridge review, not a `crates/dom` defect).

## Summary table

| # | Finding | Verdict | Severity |
|---|---|---|---|
| 1 | u32 generation wraparound reachable, silent aliasing on hit | Real | Medium-High |
| 2a | Descendant-handle staleness after adopt not explicitly tested | Real (test gap) | Low |
| 2b | adopt_node can't preserve JS object identity (spec/WPT) | Real (flag forward) | Low-Medium |
| 3a | Self-aliased args risk corrupted sibling links if commit uses cached state | Real | Medium |
| 3b | clone_node depth-guard timing — possible orphaned-slot leak | Real | Medium |
| 3c | replace_child may false-positive on "≤1 Element child" for a legit swap | Real | Medium |
| 4 | No reclaim path for detached-abandoned nodes; unbounded growth | **Real, headline** | **High** |
| 4′ | adopt_node subtree free-batching not tested | Real (test gap) | Low |
| 5a | MAX_ANCESTOR_WALK conflates hardening with correctness; unstated value | Real | Medium |
| 5b | Document child ordering not enforced | Real (minor) | Low |
| 5c | "belongs to parent" check — verify O(1) | Advisory | — |
| 5d | Revision/DocumentId wraparound, gen-0 aliasing, read-reentrancy | False alarm | — |
