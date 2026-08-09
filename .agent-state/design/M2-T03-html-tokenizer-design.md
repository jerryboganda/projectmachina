# Design: `crates/html` streaming HTML tokenizer (M2-T03)

> Produced by a wave-1 architect research agent. Read-only; no files changed.

Grounded in `planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md` (`## M2-T03`), `architecture/DOM_HTML_AND_EVENTS.md`, `OWNER_DECISIONS.md` (D01-B, clean-room/permissive-license default), `quality/WPT_PLAN.md`, `quality/FUZZING.md`, `quality/TEST_DATA.md`, `quality/FAST_INNER_LOOP.md`, ADR-006. `crates/html` is currently only `.gitkeep`, not yet a workspace member — the implementer must add `"crates/html"`.

## 0. Crate identity

Package `machina-html`, path `crates/html`. Sits below `machina-dom` (M2-T05) and `machina-native-core` — must NOT depend on `command-model`/`command-bus`/`capability`/any protocol crate. Zero required runtime deps target; `serde_json` acceptable as **build-dependency only** (entity table codegen). `fuzz/` is its own nested `Cargo.toml`, NOT a workspace member (own pinned nightly toolchain, isolated from the root `1.86.0` pin).

## 1. Tokenizer state machine (WHATWG §13.2.5)

Single flat `enum State` (~80 variants, doc-commented with spec anchors), grouped into modules: text-content modes (`Data`/`Rcdata`/`Rawtext`/`ScriptData`/`Plaintext`, selected externally per §7.2) · tag open/name · script-data escaping (12-state subgroup, isolated as most error-prone) · attributes (dedup-first-wins happens here) · comments · DOCTYPE · CDATA (foreign-content only) · character references (uses spec-defined `return_state`, `temporary_buffer`, `character_reference_code` fields).

**Rust representation:** `Tokenizer::step()` is a **flat, non-recursive** driver — no per-nesting-level call stack, so stack-overflow-via-adversarial-input is structurally impossible at this layer (unlike the tree builder, M2-T04, which needs an explicit depth limit). All state-local scratch lives as `Tokenizer` struct fields, not stack locals — this is also *why* streaming falls out for free (§3) rather than needing special-cased pause/resume logic. Dispatch is one `match self.state {...}` delegating to per-group `states/*.rs` files (each a few hundred lines, independently testable).

## 2. Character reference (entity) decoding

- Source: WHATWG `entities.json` (~2231 entries), vendored at `crates/html/data/entities.json`, pinned to a specific `whatwg/html` commit, recorded in `data/PROVENANCE.md`.
- **Licensing flag — must go to `agents/BLOCKERS.md` before merge** per D01-B ("independent clean-room implementation... final licensing requires counsel review") and `quality/TEST_DATA.md`'s per-fixture license/provenance requirement. If disallowed, the table can be regenerated independently from HTML spec §13.5's normative listing (different provenance, same data) — both paths noted in `PROVENANCE.md` so the decision is reversible.
- **Lookup:** named-char-ref matching needs longest-prefix matching (spec §13.2.5.73), ruling out a plain `HashMap`. Recommended MVP: `build.rs` emits a name-sorted `&[(&str, [u32;2], bool)]` array into `OUT_DIR`; `NamedCharacterReference` state does incremental narrowing binary search over it (binary-search-as-trie), O(log n) per character, no unsafe, no runtime dependency. A DAFSA/perfect-hash compression is a non-required fast-follow.
- Numeric refs (`&#...;`) use the spec's small fixed 32-entry C1-control remapping table.

## 3. Incremental/streaming tokenization

**Core mechanism:** because state lives in struct fields not the call stack, pausing at "no more input right now" and resuming later is the *same code path* as normal advancement — not a special case. API distinguishes `feed(&mut self, chunk: &[u8])` (never = EOF) from `finish(&mut self)` (true end of stream, triggers spec EOF actions like flushing `temporary_buffer` as literal chars).

- **Partial UTF-8 at chunk boundaries:** `InputStream` holds `pending_bytes: [u8;3]` + `pending_len`. Decode via `core::str::from_utf8`; incomplete-at-end sequences retained for next chunk; genuinely invalid bytes → U+FFFD + diagnostic (never panics, never `unsafe` transmute).
- **Partial newlines:** single `pending_cr: bool` — CRLF→LF / lone-CR→LF normalization (spec §13.2.3.5) must produce identical output regardless of where `\r\n` was split across chunks.
- **Partial tags/comments/entities:** no special handling beyond the above — `current_tag`/`temporary_buffer`/accumulating comment-data buffer just keep building across `feed()` calls, state resumes naturally.
- **Formal equivalence contract (the literal acceptance criterion):** chunked vs. unchunked input must produce equivalent token streams — non-`Character` tokens identical in order/content; `Character` tokens equivalent at the *concatenated-text* level between two non-`Character` tokens (exact token-count divergence is only sanctioned by the max-character-run flush limit, §4, set high enough — 64 KiB — that no WPT/fixture input triggers it). This exact contract is what the differential fuzz target in §5 checks.

## 4. Bounded-input / crash-free behavior

Structural: flat FSM (§1) has zero adversarial-recursion surface by construction. Every remaining unbounded-growth surface gets an explicit configurable `TokenizerLimits`, each with a **defined recovery action** (truncate/skip/resync + `Diagnostic`), never an abort:

| Field | Default | Recovery |
|---|---|---|
| `max_tag_name_len` | 4 KiB | truncate, keep scanning for `>` |
| `max_attribute_count` | 512/tag | stop adding, still parse/discard remainder to avoid desync |
| `max_attribute_value_len` | 8 MiB | truncate (generous — legit `data:` URIs exist) |
| `max_comment_len` | 16 MiB | truncate, keep scanning for `-->` (classic `<!--` with no close DoS) |
| `max_doctype_field_len` | 64 KiB | truncate |
| `max_character_run_len` | 64 KiB | forced flush — the one legitimate chunk-boundary-dependent token *count* divergence |
| `max_tag_or_comment_byte_span` | 8 MiB | hard backstop distinct from per-field limits (catches e.g. huge attribute *count* each individually under-limit); force-terminate as bogus/oversized, resync to `Data` |
| `max_named_char_ref_scan_steps` | 64 | defense-in-depth, independent of table's own max name length |
| `max_diagnostics_buffered` | 4096/call | coalesce into one `TooManyDiagnostics` summary past this, prevents unbounded diagnostics Vec on "every byte is an error" input |

No `unwrap`/`expect`/unchecked cast/`unsafe` anywhere (matches `AGENTS.md`). Numeric char-ref accumulation uses saturating `u32` arithmetic, clamped at the spec sentinel (`>0x10FFFF` → invalid → U+FFFD) — no overflow panic path.

## 5. Fuzz target design

`crates/html/fuzz/` (cargo-fuzz, not a workspace member). Two targets, both linking the **production** tokenizer directly (no divergent fuzz-only path, per `quality/FUZZING.md`):
- `tokenize_whole.rs` — feed random bytes, drain, must never panic/hang.
- `tokenize_chunked.rs` — the §3 equivalence property itself: run the same bytes whole vs. arbitrarily chunked, assert equivalent per the concatenated-text contract.

Fast-gate smoke: `cargo fuzz run tokenize_chunked -- -max_total_time=60 -rss_limit_mb=2048 -timeout=5`; longer hours-scale runs deferred to scheduled/M8/M9 per ADR-006.

**Curated corpus** (`fuzz/corpus/tokenizer/`): every `input` field extracted from the vendored html5lib-tests JSON files (§6) as seeds, plus hand-authored adversarial cases (unterminated tag/comment/doctype/CDATA at EOF; 10,000+ attributes; multi-MiB unclosed attribute value; deeply nested `<!--<!--<!--`/script-double-escape nesting; ambiguous entities `&amp`/`&ampx;`/`&notit;` vs `&notin;`/`&#x;`/numeric overflow/lone `&`; NUL bytes in every state; invalid UTF-8 incl. truncated multi-byte, overlong encodings, lone continuation bytes, BOM; byte-at-a-time feed of a large well-formed doc). `fuzz/regressions/` retains minimized historical crashes permanently per `quality/TEST_DATA.md`.

## 6. WPT/html5lib-tests tokenizer subset

Vendor from `web-platform-tests/wpt` path `html/syntax/parsing/resources/html5lib-tests/tokenizer/` (**verify exact file list against the actually-pinned commit, don't trust blindly** — upstream can rename/add files): `contentModelFlags.test`, `domjs.test`, `entities.test`, `escapeFlag.test`, `namedEntities.test`, `numericEntities.test`, `pendingSpecChanges.test`, `test1-4.test`, `unicodeChars.test`, `unicodeCharsProblematic.test`, `xmlViolation.test`.

Lives at `tests/wpt/html/tokenizer/` (mirrors `tests/fixtures/manifest.json`'s conventions from M0-T08) with `LICENSE`, `PROVENANCE.md` (upstream repo/commit/date/license), `manifest.json` (per-file sha256, revision pin, P0/P1 tier, owner), `expected_failures.json` (known-failing case IDs with owner+issue link, per `quality/WPT_PLAN.md` — "do not blanket-skip directories").

**Harness** (`crates/html/tests/html5lib_tokenizer.rs`, `serde_json` dev-dependency): deserialize `tests` array per file; map `initialStates`/`lastStartTag` to tokenizer seed calls via the same external state-switch hook (§7.2); handle `doubleEscaped` unescaping; map `["DOCTYPE",...]`/`["StartTag",...]`/`["EndTag",...]`/`["Comment",...]`/`["Character",...]` JSON entries to `Token` variants (Character comparison at the *concatenated* level, not 1:1, per §3's contract — html5lib-tests itself sometimes splits/merges runs); compare `errors` **presence-only** for M2-T03 (full code-parity is a tracked follow-up, not day-one acceptance bar). Fast gate runs a P0 shard (`test1.test`, `entities.test`, `namedEntities.test`, `numericEntities.test`, `contentModelFlags.test`); the rest deferred to scheduled/M8/M9.

## 7. Public API surface exposed to M2-T04

```rust
#[non_exhaustive]
pub enum Token { Doctype(DoctypeToken), StartTag(TagToken), EndTag(TagToken), Comment(CommentToken), Character(CharacterToken), Eof }
pub struct TagToken { pub name: String, pub self_closing: bool, pub attributes: Vec<Attribute> } // dedup already applied, first-wins
pub struct Attribute { pub name: String, pub value: String }
pub struct DoctypeToken { pub name: Option<String>, pub public_id: Option<String>, pub system_id: Option<String>, pub force_quirks: bool }
```
`String`/`Vec` deliberately used for MVP correctness-first; small-string/interning is a tracked non-blocking perf follow-up (interning itself deferred to `machina-dom`, M2-T05).

**Key interface point — external state-switch hook.** The tokenizer alone can't know from a bare `<title>`/`<script>` start tag whether to switch to RCDATA/RAWTEXT/script-data — that decision belongs to tree construction, which must call back immediately after receiving such a token, before the next byte is processed:
```rust
pub fn switch_to(&mut self, state: TextContentState); // called by M2-T04's tree builder
pub fn last_start_tag_name(&self) -> Option<&str>;    // for "appropriate end tag" logic
```

**Pull API:** `new(limits)`, `feed(chunk)`, `finish()`, `switch_to()`, `last_start_tag_name()`, `next_event() -> Option<TokenizerEvent>` where `TokenizerEvent::{Token(Token), Diagnostic(Diagnostic)}` (interleaved so error positions correlate with the token being built).

**Diagnostics are deliberately NOT the `ERROR_MODEL.md` canonical `{code,category,retryable,...}` shape** — that's for command-bus/protocol-facing failures. `Diagnostic{code: ParseErrorCode, position: u64 (stream-wide byte offset), severity}` is a local, allocation-light, non-fatal event type (~60 codes mirroring WHATWG §13.2.2's named parse-error list, plus Machina-specific `LimitExceeded(LimitKind)`/`TooManyDiagnostics`). A higher layer (native-core or the tree builder) decides whether any of this becomes a canonical error — keeps the parser decoupled from protocol concerns.

## Traceability to acceptance criteria

Priority WPT/fixtures pass → §6 harness+shard · arbitrary chunk boundaries equivalent → §3 design + §5 `tokenize_chunked` fuzz target · adversarial input bounded/crash-free → §4 limits + no-unsafe + flat-FSM invariant + §5 fuzz corpus · incremental tokens/diagnostics without panics → §7 pull API · fast gate (WPT shard, fuzz seed smoke) → §6.4, §5.

## Open items requiring human/legal sign-off (not blocking the design, but must be flagged)

- `data/entities.json` provenance/license — flag in `agents/BLOCKERS.md` before merge (D01-B).
- Exact WPT/html5lib-tests commit pin + file list must be confirmed live at vendoring time.
- Root `Cargo.toml` needs `"crates/html"` added to workspace members; `crates/html/fuzz` deliberately excluded.

## Files reviewed

`planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md` · `architecture/DOM_HTML_AND_EVENTS.md` · `architecture/NATIVE_ENGINE.md` · `architecture/ERROR_MODEL.md` · `architecture/REPOSITORY_STRUCTURE.md` · `OWNER_DECISIONS.md` · `quality/WPT_PLAN.md` · `quality/FUZZING.md` · `quality/TEST_DATA.md` · `quality/FAST_INNER_LOOP.md` · ADR-006 · `crates/native-core`, `crates/command-model`, `crates/telemetry` (convention check).
