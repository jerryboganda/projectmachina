# Design: M2-T10 — CSS Selector Queries and Initial XPath

> Produced by a wave-2 architect research agent. Read-only; no files changed.

## 0. Crate placement — `crates/selectors`, new leaf crate, NOT folded into `crates/dom`

`crates/selectors` (package `machina-selectors`), depends only on `machina-dom` (path dep) + std. Does not exist yet — add `"crates/selectors"` to workspace `members`. Resolves a real tension: `REPOSITORY_STRUCTURE.md`'s illustrative tree annotates `crates/dom/` as "nodes, mutation, selectors, ranges," but the concrete, current `.agent-state/design/M2-T05-dom-design.md` explicitly designs `dom` as a "leaf crate so M2-T04/T06/T07/T10 can all depend on it without cycle risk" and names selectors/XPath as M2-T10's job, not dom's. Resolved in favor of separate crate: (1) the M2-T05 design is the specific, current authority dom is actually being built against; (2) AGENTS.md's multi-agent non-overlapping-ownership rules — M2-T05 and M2-T10 are separate tasks with separate fast gates; (3) keeps `dom`'s "no deps beyond std" leaf property and its own already-large test/fuzz matrix from growing further; (4) `crates/selectors` has no `.gitkeep` stub unlike other planned crates — consistent with being freshly created by this task, same as `dom`/`html` were.

Bundles both CSS selectors and XPath as internal modules (`css/`, `xpath/`) in one crate — one task, one fast gate, shared infrastructure (document-order walk, `QueryResult`/`QueryError`, depth limits).

**Coordination note for M2-T05:** `dom::arena::MAX_ANCESTOR_WALK` is currently `pub(crate)`; `crates/selectors` needs an equivalent bound for its own ancestor/sibling walks. Recommend `dom` exports it publicly (small low-risk M2-T05 follow-up); fallback is `selectors` mirroring the same numeric value in its own `limits.rs` with a cross-reference comment so they don't silently drift.

## 1. Selector grammar subset

**In scope (priority):** type/universal/class/id selectors; all 6 attribute operators (`[attr]`, `=`, `~=`, `^=`, `$=`, `*=`, `|=` — cheap, no regex needed, and `^=`/`*=`/`~=` are what automation selectors like `[data-testid^="row-"]` actually lean on); descendant/child/adjacent-sibling/general-sibling combinators; selector lists (`,`); `:first-child`/`:last-child`/`:only-child`/`:nth-child(an+b)`/`:nth-last-child(an+b)`/`:empty`/`:root`/`:not(<compound>)` (included despite complexity — reduces to "match compound, negate," extremely common in real selectors).

**Explicitly deferred, not silently unsupported** (each gets a distinct `UnsupportedFeature` error, never silent no-op): live/interaction pseudo-classes (`:hover`/`:focus`/`:active`/`:visited`/`:target` — no layout/interaction-state model yet); `:has()`/`:is()`/`:where()` (flagged as low-risk fast-follows once the matcher exists); `:lang()`; namespace selectors; pseudo-elements; `:first-of-type` family (same complexity as included nth-child family, lower observed frequency); case-insensitive attribute flag `[attr=val i]`; shadow-piercing selectors (Shadow DOM is M3-T04).

Case sensitivity: tag/attribute *names* are ASCII-case-insensitive for HTML, inherited for free from M2-T04's tree builder lowercasing during parse (matcher documents this, doesn't re-implement folding). Attribute *values* match case-sensitively (CSS/HTML default).

## 2. Matcher: right-to-left, bounded backtracking ancestor walk

Compiled `Selector` is a reversed sequence — rightmost compound first (`a > b c` compiles to `[c, (child)b, (descendant)a]`). Match algorithm (standard production-engine approach — Blink/WebKit/Servo — reimplemented clean-room per D01-B, not vendored):
1. Test only the rightmost compound against candidate `e` first — reject immediately with zero tree traversal if it fails.
2. If it passes with no combinator, match succeeds.
3. Otherwise walk left per combinator: descendant → try each ancestor in turn, backtracking (a failed rest-of-chain at one ancestor doesn't stop the walk); child → test exactly `parent`, no walk; adjacent-sibling → test exactly `previous_sibling`; general-sibling → try each previous sibling, same backtracking shape as descendant.
4. Walk depth bounded by the same guard class as M2-T05's `MAX_ANCESTOR_WALK` — adversarially deep/wide trees fail closed with `QueryError::TooComplex`, never unbounded work or stack overflow.

Compound-match order is cheapest-first: id/type (single `Atom` comparison) → class/attribute-presence (linear `AttributeMap` scan) → attribute-value operators (string compare) → structural pseudo-classes (`:nth-child` etc. — sibling-position counting, most expensive, ordered last so plain mismatches never pay for it).

**Right-to-left justification:** automation selectors typically pair a specific/unique rightmost class-or-attribute with a broad left context (`div p .title`) — right-to-left rejects most non-matching candidates in O(1) before touching tree structure at all. Left-to-right would lose that fast-reject property.

## 3. Query indexes: plain tree-walk MVP, indexing explicitly deferred

**No incrementally-maintained index (id/class/tag) for M2.** `query_selector_all` is a pre-order DFS walk testing each candidate via §2's matcher. `getElementById`/`getElementsByClassName`/`getElementsByTagName` reuse the same walk with a specialized single-simple-selector fast path, no separate index either.

Justification: M2 has no layout/large-document-heavy workload in scope (perf/load is scheduled M8/M9); an incrementally-maintained index would need its own mutation-observer hook parallel to `WrapperObserver`, correct across every mutation path M2-T05 already spent significant effort validating, and its own teardown — doubling the exact stale/inconsistent-derived-state bug class M2-T05's design eliminated for the tree itself. `Document::revision()` already delivers most of the practical caching benefit via a caller-side `(selector, revision)` cache (§4) without a second data structure to keep consistent. Deferred as a documented future optimization (first candidate if profiling shows it matters: `getElementById` — best complexity/ROI, O(1) win, single clear invalidation trigger).

## 4. Cache invalidation via `Revision`

Two distinct layers: (a) compiled-selector cache is pure syntax → AST, `Document`/`Revision`-agnostic, safely reusable across documents/time (caller's own optional cache — `crates/selectors` stays stateless). (b) `QueryResult{revision, elements}`/`XPathResult{revision, items}` self-describe the `Revision` they were computed against — always a fresh tree walk, stamped with `document.revision()` at completion. Staleness detection is plain `result.revision != document.revision()`, never guessed/implicit — this is the literal, mechanically-testable meaning of "live document revisions are correct." `crates/selectors` doesn't own an internal result cache (same "thin layer on top" choice M2-T05 made). Inherited trade-off from M2-T05's coarse (non-split) revision counter: any mutation anywhere invalidates every cached result conservatively (correct, not maximally precise) — not re-litigated here, a documented fast-follow only if profiling shows it matters.

## 5. XPath scope

**In:** absolute/relative location paths, `//` abbreviation; axes `child::`(default)/`descendant::`(`//`)/`attribute::`(`@`)/`self::`(`.`)/`parent::`(`..`); node tests: element name, `*`, `text()`, `node()`, `comment()`; predicates: positional `[N]`, `[@attr]`, `[@attr='v']`, `[last()]`, `and`-combinations. Result: ordered node-set only, document order — no boolean/number/string top-level coercion (caller composes `evaluate_xpath` + `.text_content()` from `dom`).

**Out:** `following-sibling::`/`preceding-sibling::`/`following::`/`preceding::`/`ancestor::`/`namespace::` axes; `processing-instruction()`; `contains()`/`starts-with()`/`substring()`/general boolean-arithmetic/`or`/XPath `not()`/unions (`|`) — `contains()`/`starts-with()` flagged as the most plausible near-term fast-follow (common in hand-written automation scripts like `//button[contains(text(),'Submit')]`) but not required now; full XPath 2.0/3.1, variables, function library.

Justification against over-scoping into M2-T13/M3-T02: M2-T13's semantic index derives roles/names via its own traversal + `query_selector_all`, not ad hoc XPath predicates. M3-T02's locator resolution is described as semantic/DOM-locator-centered (role/text/test-id), with XPath mainly a compatibility escape hatch for callers supplying literal XPath strings — structural path + simple predicates cover that.

**Attribute-axis result gap this design closes:** M2-T05's DOM has no attribute `NodeKind` (`AttributeMap` is inline `Vec<(Atom, Box<str>)>`, not arena nodes) — so `//div/@id` can't resolve to a `NodeHandle`. Result type:
```rust
pub enum XPathItem { Node(NodeHandle), Attribute { owner: ElementHandle, name: String, value: String } }
pub struct XPathResult { pub revision: Revision, pub items: Vec<XPathItem> }
```
Context node: `document.evaluate_xpath(expr, context: Option<NodeHandle>)`. Absolute paths ignore `context`; relative paths with `context: None` → `QueryError::ContextNodeRequired`, never silently defaults to document root (would mask a caller bug — relevant for M3-T02's per-element locator resolution later).

## 6. Canonical query outcomes and error handling

```rust
pub struct QueryResult { pub revision: Revision, pub elements: Vec<ElementHandle> }
#[non_exhaustive]
pub enum QueryError {
    InvalidSelector { message: String, position: usize },
    InvalidXPath { message: String, position: usize },
    UnsupportedFeature { feature: String, position: usize },
    TooComplex { limit: LimitKind },
    ContextNodeRequired,
    DomError(DomError), // via From<DomError>
}
```
Three-way split enforced explicitly: **legitimate empty match is `Ok(vec![])`, not an error** (dedicated type-level test). **Malformed syntax is always a typed parse error, staged strictly before matching** (two-phase, mirrors M2-T05's validate-then-commit — invalid syntax never reaches the tree walk, can never "partially match"). **Valid-but-out-of-M2-scope constructs get `UnsupportedFeature`, distinct from both** — this is the mechanism preventing the most dangerous silent-failure mode for an automation product: a selector that silently dropped an unsupported clause and matched something *else* than intended (wrong action target) would be far worse than a loud refusal. No panics/`unwrap`/`unsafe` anywhere — hand-written recursive-descent parser (clean-room, no external selector/XPath crate, matching M2-T03's posture), `Result` at every production. `TooComplex` is the explicit typed alternative to unbounded recursion (mirrors `dom`'s `HierarchyViolation`/`MAX_ANCESTOR_WALK` and `html`'s `TokenizerLimits`). Stale/cross-document/closed handles delegate to `dom::DomError` via `From`, not reinvented.

## 7. Test strategy → acceptance criteria

Priority WPT fixtures → `tests/css_fixtures.rs` against a vendored curated CSS-selectors/`ParentNode-querySelector(All)` slice at `tests/wpt/selectors/`, reusing M2-T03's exact manifest/provenance/expected-failures pattern (not re-derived). Query order + live revisions → `tests/matcher_order.rs` (document-order + backtracking-combinator correctness) + `tests/revision_invalidation.rs` (query→mutate→requery, assert revision strictly increases and reflects post-mutation tree). Invalid expressions never crash/silently match → `tests/errors.rs` (malformed strings → typed parse errors, never panic; out-of-scope-but-valid constructs → `UnsupportedFeature`, never empty `Ok`; legitimate zero-match → explicit separately-named `Ok(vec![])` test distinguishing it from the error cases). XPath subset → `tests/xpath_fixtures.rs`. Fuzz fast gate → `crates/selectors/fuzz/` (own nested Cargo.toml, not a workspace member, mirrors `crates/html/fuzz` exactly): `parse_selector.rs`/`parse_xpath.rs` targets, `cargo fuzz run parse_selector -- -max_total_time=60 -rss_limit_mb=2048 -timeout=5`, hand-authored seed corpus (unterminated brackets, `:not()` nesting to the depth-guard boundary, combinator-only strings, embedded NUL/non-UTF-8 bytes, etc.). "Differential checks" fast gate scoped to fixture-expected-output comparison (native vs. pre-recorded expected element list) — NOT a live second-browser-engine differential, which stays deferred to M2-T14/M8-M9 per AGENTS.md's quality strategy.

## 8. Module layout

```
crates/selectors/  Cargo.toml (path-dep on machina-dom only, no deps beyond std)
  src/ lib.rs ·
       css/ mod.rs · tokenizer.rs · parser.rs (two-phase, Err never leaves a partial AST) ·
            ast.rs (immutable, Document-agnostic, safely cacheable) ·
            matcher.rs (right-to-left core, §2) · pseudo.rs (:nth-child family etc.) ·
       xpath/ mod.rs · tokenizer.rs · parser.rs (§5 grammar only) · ast.rs · evaluator.rs ·
       query.rs (shared document-order DFS walk used by both query_selector_all and XPath axis stepping;
                  Document-facing entry points) ·
       error.rs (QueryError, hand-written, From<DomError>) ·
       limits.rs (QueryLimits — coordinates with dom::MAX_ANCESTOR_WALK, §0)
  fuzz/ Cargo.toml (not a workspace member) · fuzz_targets/{parse_selector,parse_xpath}.rs ·
        corpus/{selectors,xpath}/ · regressions/ (permanent, per TEST_DATA.md)
  tests/ css_fixtures.rs · xpath_fixtures.rs · matcher_order.rs · revision_invalidation.rs · errors.rs
tests/wpt/selectors/  manifest.json · PROVENANCE.md · expected_failures.json · LICENSE (mirrors tests/wpt/html/tokenizer/)
```
Root `Cargo.toml`: add `"crates/selectors"` to workspace members (crate doesn't exist yet).

## Files reviewed

`AGENTS.md` · `.agent-state/design/M2-T05-dom-design.md` · `.agent-state/design/M2-T03-html-tokenizer-design.md` · `planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md` · `planning/MILESTONE_03_NATIVE_WEB_APIS_AND_AUTOMATION.md` (M3-T02/M3-T04 forward-scoping only) · `planning/MASTER_TASK_GRAPH.md` · `architecture/REPOSITORY_STRUCTURE.md` · `architecture/DOM_HTML_AND_EVENTS.md` · `OWNER_DECISIONS.md` (D01-B) · `quality/FUZZING.md`, `WPT_PLAN.md`, `FAST_INNER_LOOP.md` · root `Cargo.toml` + crates/ listing (confirmed `crates/selectors` doesn't exist yet).
