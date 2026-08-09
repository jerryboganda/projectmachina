# Design: M2-T04 — HTML Tree Builder and Document Construction

> Produced by a wave-2 architect research agent. Read-only; no files changed.

## 0. Crate identity — new sibling crate `crates/html-tree-builder`, NOT a module in `crates/html`

Package `machina-html-tree-builder`, depends on `machina-html` + `machina-dom` (no protocol/command crates). Reasoning against folding into `crates/html`: (1) dependency direction — `crates/dom` is a deliberate leaf and `crates/html` is contracted as zero-runtime-deps; a tree-builder module inside `html` would force it to depend on `dom`, breaking an already-adopted contract. (2) AGENTS.md's hard ownership-non-overlap rule — two builder agents (T03, T05) are active right now; a module inside `html` would put T04 editing files T03 is actively writing. (3) Independent convergent precedent — the wave-1 WPT/benchmark harness plan already names `machina-html-tree-builder` as a distinct crate, written by a separate agent with no visibility into this decision. (4) This crosses from "no deps" to "depends on two sibling crates" — a different shape than `crates/dom` absorbing T05+T10 (both pure extensions of the same leaf arena). Reversible, internal build-organization choice, no ADR needed; `REPOSITORY_STRUCTURE.md`'s `html/ # tokenizer/tree builder` shorthand comment should be split as a non-blocking doc-sync follow-up.

```toml
[dependencies]
machina-html = { path = "../html" }
machina-dom  = { path = "../dom" }
[dev-dependencies]
machina-wpt-support = { path = "../wpt-support" }
```
All three of `html`/`dom`/`html-tree-builder` still need adding to root `Cargo.toml` workspace members — flagged as a shared-file coordination point across the concurrently active T03/T05 builders.

## Required interface additions to M2-T05 (blocking coordination — relay to the T05 builder)

Everything else in M2-T05's public surface is used as-is, unmodified. Two gaps need closing, ideally by M2-T05's own implementation directly:

1. **Namespace-aware element creation** — M2-T05 §7 explicitly deferred this ("data model leaves room for a namespace Atom field but doesn't build it out"). T04 needs it built:
   ```rust
   pub enum Namespace { Html, Svg, MathMl }
   impl Document {
       pub fn create_element_ns(&mut self, namespace: Namespace, local_name: &str) -> Result<ElementHandle, DomError>;
       pub fn element_namespace(&self, handle: ElementHandle) -> Result<Namespace, DomError>;
   }
   ```
   `create_element` (HTML-only) can remain as sugar for `create_element_ns(Namespace::Html, name)`.
2. **DocumentType construction** — `NodeKind` already lists `DocumentType` but M2-T05's constructor list doesn't include one:
   ```rust
   pub struct DocumentTypeHandle(NodeHandle);
   impl Document {
       pub fn create_document_type(&mut self, name: &str, public_id: &str, system_id: &str) -> Result<DocumentTypeHandle, DomError>;
   }
   ```
   (The "Document accepts ≤1 DocumentType child" cap logic already exists per M2-T05 §2 — just needs the constructor.)

Both additive, no signature changes, fit the arena/generational-handle model as-is.

## 1. Insertion modes

Flat `enum InsertionMode` (23 WHATWG variants, doc-commented spec anchors) — same flat-FSM posture as the tokenizer. Fields: `mode`, `original_mode: Option<InsertionMode>` (spec's "original insertion mode" for RCDATA/RAWTEXT/InTableText), `template_modes: Vec<InsertionMode>` (spec's "stack of template insertion modes"). Dispatch is one `loop { match mode {...} }` per `process_token`, not recursive mode-handler calls — "reprocess under a different mode" returns `Dispatch::Reprocess(new_mode)` consumed by the outer loop, bounded by a small hop counter (§7f) instead of a real call stack, keeping stack depth O(1) regardless of reprocess count.

## 2. Stack of open elements / active formatting elements — built directly on M2-T05 handles

No parallel node-identity type. Both store `ElementHandle` plus a small cache captured at push time from the creating token, so scope/tag comparisons during hot recovery algorithms never round-trip through the DOM:
```rust
struct OpenElementEntry { handle: ElementHandle, tag: Atom, namespace: Namespace }
pub struct OpenElementsStack(Vec<OpenElementEntry>);
```
Methods include per-scope-kind checks (`has_element_in_scope`, `..._in_list_item_scope`, `..._in_button_scope`, `..._in_table_scope`, `..._in_select_scope`) — each a top-down walk of cached tag/namespace pairs, no DOM dereference except at actual mutation time.
```rust
enum FormattingEntry { Marker, Element { handle, tag, namespace, attrs: Vec<(Atom, Box<str>)> } }
pub struct ActiveFormattingElements(Vec<FormattingEntry>);
```
`attrs` is the original start-tag's attributes snapshot — needed because AAA's "create an element for the token" step must recreate a formatting element with the *original* attributes, not the live DOM element's current ones — exactly why AFE entries can't be "just a handle." Methods: `push_with_noahs_ark` (Noah's Ark clause, compared on cached snapshot only), `insert_marker`, `clear_up_to_last_marker`, `reconstruct_active_formatting_elements`.

## 3. Foster parenting + adoption agency — compose the existing M2-T05 mutation API, no new DOM primitive

**Foster parenting**: a pure targeting function returning `(ElementHandle, Option<NodeHandle>)`, consumed by every insertion-mode handler that needs it (`InTable`, `InTableText`, `InCaption`, `InTableBody`, `InRow`) via one shared code path, not duplicated per mode. Scans the open-elements stack top-down for the last `template`/`table` per spec; branches to `insert_before` or `append_child` accordingly.

**Adoption agency (AAA)**: one function following the WHATWG numbered steps literally. Bookmark = a plain index into the AFE `Vec`, adjusted at every insert/remove (standard technique). Reparenting reuses the same foster-parent helper when the common ancestor is table-context (spec explicitly says foster-parenting applies inside AAA too — one shared implementation). Cloning a misnested formatting element uses the AFE entry's cached tag/namespace/attrs via the same generic "insert an element" helper — deliberately NOT `Document::clone_node` (that clones a live subtree; AAA needs a fresh element with the *original token's* attributes only). Loop bounds are spec-fixed and small (outer ≤8, inner ≤3) — enforced as literal counter checks, not left implicit.

## 4. Foreign content (SVG/MathML)

Element creation always via `create_element_ns` once foreign-content dispatch is active. Three small static tag/attribute-adjustment tables (SVG tag-case, SVG attribute-case, foreign attribute namespaces like `xlink:href`) vendored the same way as the tokenizer's entity table — build-time-generated, no runtime dependency. Since M2-T05's `AttributeMap` is namespace-agnostic, namespaced attributes use a compound interned name (`xlink:href` as one atom) as an explicit MVP simplification — revisit only if `getAttributeNS`-fidelity WPT tests later require it.

Integration-point predicates over cached tag+namespace (MathML text-integration: `mi/mo/mn/ms/mtext`; HTML-integration: `annotation-xml` with cached `encoding` = `text/html`/`application/xhtml+xml`, or SVG `foreignObject/desc/title`). Dispatch rule computes the "adjusted current node" before each token per spec, routing through ordinary HTML insertion modes or the separate `foreign.rs` foreign-content rules (not folded into `InsertionMode` — it's a cross-cutting override, matching spec structure). Fragment/context-element support via `TreeBuilder::new_fragment(context, limits, scripting_enabled)`.

## 5. Driving the tokenizer's text-content state (M2-T03's `switch_to`/`last_start_tag_name` hooks)

| Start tag (HTML ns only) | Call | Algorithm |
|---|---|---|
| `title`, `textarea` | `switch_to(Rcdata)` | generic RCDATA |
| `style`, `xmp`, `iframe`, `noembed`, `noframes` | `switch_to(Rawtext)` | generic RAWTEXT |
| `noscript` (scripting enabled only) | `switch_to(Rawtext)` | same; scripting disabled → ordinary HTML content, no switch |
| `script` | `switch_to(ScriptData)` | tokenizer's own script-data-escaped sub-states are entered/exited internally purely from byte sequences — tree builder makes exactly this one call |
| `plaintext` | `switch_to(Plaintext)` | one-way, spec defines no exit |

Sequence: insert element normally (on open-elements stack + in DOM before content arrives) → `switch_to` → `original_mode = Some(mode)` → `mode = Text`. Exit on matching end tag (via `last_start_tag_name()`): pop, restore `mode = original_mode.take().unwrap_or(InBody)`. **Applies only to ordinary HTML-namespace insertion** — foreign-content insertion (e.g. an SVG `<script>`) never calls `switch_to`, deliberate spec-matching asymmetry, documented inline so it isn't "fixed" by mistake later.

## 6. Parser-blocking script checkpoints — pause/resume contract for M2-T09

`TreeBuilder` never owns `Document` — every driving method takes `&mut Document` per call (handles are `Copy`, so no live borrow persists between calls). Pausing is just "return from `feed()`" — no self-referential struct, no lifetime gymnastics.

```rust
pub enum TreeBuilderOutcome { NeedsMoreInput, ScriptCheckpoint(ScriptCheckpoint), Done }
pub struct ScriptCheckpoint { pub script_element: ElementHandle, pub source: ScriptSource } // Inline | External
pub fn feed(&mut self, doc: &mut Document, tokenizer: &mut Tokenizer, chunk: &[u8]) -> Result<TreeBuilderOutcome, TreeBuilderError>;
pub fn finish(&mut self, doc: &mut Document, tokenizer: &mut Tokenizer) -> Result<TreeBuilderOutcome, TreeBuilderError>;
pub fn resume_after_script(&mut self, doc: &mut Document, tokenizer: &mut Tokenizer) -> Result<TreeBuilderOutcome, TreeBuilderError>;
```

`feed`/`finish` loop internally: push bytes → tokenizer → pull events → `process_token` → stop at `NeedsMoreInput`/`Done`/a `</script>` end tag closing an HTML-namespace script (`ScriptCheckpoint`). T04's job is to **track** checkpoints, not execute — every `</script>` unconditionally produces one (conservative; no async/defer/module classification, that's a future scripting-engine decision). Element + child text node are fully constructed before the checkpoint returns, matching spec ordering. **Misuse is a typed error**: `feed`/`finish` while paused → `AlreadyPaused`; `resume_after_script` while not paused → `NotPaused` — exercisable and testable now even with no production caller yet. **Resumability contract**: any `Ok` leaves `TreeBuilder` fully resumable (malformed HTML never produces `Err`, only diagnostics); `Err` is reserved for genuine internal-invariant violations and poisons the instance (caller must discard, matches M2-T03's protocol-agnostic `Diagnostic` design). **`document.write()` seam reserved but not built**: `resume_after_script` is shaped so an `injected: Option<&[u8]>` param can be added later without a contract-breaking change.

## 7. Malformed-nesting recovery — concrete defensive patterns

(a) **Tree builder enforces its own strictly-tighter depth ceiling before any DOM call** — `TreeBuilderLimits.max_open_elements_depth` with a compile-time/unit-test invariant `assert!(default().max_open_elements_depth < machina_dom::MAX_ANCESTOR_WALK)`. This makes `DomError::HierarchyViolation` provably unreachable on any path the tree builder takes, not just "shouldn't happen abstractly."
(b) Adversarial same-tag nesting (millions of unclosed `<div>`) fails closed at the tree-builder layer *before* the DOM is touched — emits `Diagnostic::NestingLimitExceeded`, treats the token as a failed-open, continues parsing the remainder under the current stack (documented, deterministic, spec-deviating recovery — the spec itself has no numeric limit). Directly satisfies "deep/adversarial nesting limit tests."
(c) Any unexpected `Err` from a DOM call the tree builder believed valid is converted to `TreeBuilderError::Internal(...)` and poisons the instance — never a "recover from the recovery algorithm" retry, which would risk infinite loops.
(d) AAA's own loop bounds are already spec-fixed and small (§3) — no extra guard needed.
(e) Implicit-close cascades only ever shrink the stack — inherit (a)/(b)'s bound automatically.
(f) Iterative dispatch, not recursive — "reprocess the token" is a bounded loop (`max_reprocess_hops`, e.g. 8), matching the DOM crate's iterative `clone_node` and the tokenizer's flat-FSM posture — stack-overflow-via-adversarial-input structurally impossible here too, same pattern one layer down and one layer up.

## 8. Test strategy → acceptance criteria

Uses Lane A (`tests/html5lib-tests/tree-construction/*.dat`) + `crates/wpt-support` exactly per the WPT harness plan — adds nothing to vendoring, only specifies consumption. `tests/html5lib_tree_construction.rs` (feature `wpt-subset`): deserializes `.dat` blocks, `#document-fragment` drives `new_fragment`, `#script-on`-only cases recorded in `tests/wpt/selection/M2-T04.selection.yaml`'s `excluded_but_in_priority_tier` with reason "no scripting engine until M2-T08/T09" (never silently skipped). A test-only tree serializer renders built DOM into `.dat` format for comparison, aggregated-failure reporter into `artifacts/wpt/M2-T04/`. P0 shard: `tests1-4.dat`, `tables01.dat`, `adoption01/02.dat`, `foreign-fragment.dat`, `svg.dat`, `math.dat`, `template.dat`. `tests/adversarial_nesting.rs` (default fast gate, Machina-authored): deep unclosed nesting hits the limit deterministically (never panic, never `HierarchyViolation` escaping `Document`); same document whole vs. byte-at-a-time produces identical tree+diagnostics (chunk-equivalence extended one layer up from M2-T03). `tests/script_checkpoint.rs` (default fast gate): checkpoint fires at exactly each script's end tag with correct DOM state, resume continues correctly, out-of-turn calls return typed errors, different chunk splits produce equivalent checkpoint positions/final tree. `tests/public_api_shape.rs`: compile-only smoke confirming no accidental protocol-crate leakage and `TreeBuilder: Send`.

## 9. Module layout

```
crates/html-tree-builder/  Cargo.toml
  src/ lib.rs · modes.rs (InsertionMode) · builder.rs (dispatch loop, new/new_fragment) ·
       open_elements.rs · active_formatting.rs · adoption_agency.rs (isolated per T03's precedent) ·
       foster_parent.rs (shared by table modes + AAA) ·
       insertion/{initial,before_html,before_head,in_head,after_head,in_body,text,in_table,
                  in_select,in_template,after_body,in_frameset,after_after_body}.rs ·
       foreign.rs · tokenizer_bridge.rs (§5 call sites) · checkpoint.rs (§6) ·
       limits.rs (const invariant vs machina_dom::MAX_ANCESTOR_WALK) · diagnostics.rs · error.rs · fragment.rs
  tests/ html5lib_tree_construction.rs (wpt-subset) · adversarial_nesting.rs · script_checkpoint.rs ·
         foreign_content.rs · public_api_shape.rs
```

## Open items / risks

- **Two additive M2-T05 interface changes needed** (`create_element_ns`/`element_namespace`, `create_document_type`) — flag to the M2-T05 builder now, not discovered at integration time.
- `REPOSITORY_STRUCTURE.md`'s `html/` line needs splitting — non-blocking doc-sync follow-up.
- Root `Cargo.toml` needs all three crates added to workspace members — shared-file edit across concurrently active T03/T05 work.
- Foreign-attribute namespace compound-atom simplification — documented MVP scope.
- `#script-on` html5lib-tests cases excluded with recorded reason/milestone, must appear in the selection yaml.

## Files reviewed

`planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md` (T04 + neighbors) · `architecture/DOM_HTML_AND_EVENTS.md`, `NATIVE_ENGINE.md`, `ERROR_MODEL.md`, `REPOSITORY_STRUCTURE.md` · ADR-001 · `AGENTS.md` · `.agent-state/design/M2-T03-html-tokenizer-design.md`, `M2-T05-dom-design.md`, `M2-WPT-and-benchmark-harness-plan.md` · root `Cargo.toml` · `crates/html/`, `crates/dom/` (confirmed `.gitkeep`-only at review time).
