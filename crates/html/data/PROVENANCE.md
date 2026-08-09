# Named-character-reference data provenance (M2-T03)

## What was vendored

A **hand-curated subset of 63 named character references** (out of the
WHATWG table's ~2231 total), covering the highest-frequency entities:
`amp`/`AMP`, `lt`/`LT`, `gt`/`GT`, `quot`/`QUOT`, `apos`, `nbsp`, `copy`/
`COPY`, `reg`/`REG`, plus common punctuation (`hellip`, `mdash`, `ndash`,
curly quotes, `bull`, daggers), a handful of Latin-1 symbols (`deg`,
`plusmn`, `times`, `divide`, `sect`, `para`, `middot`, guillemets, currency
signs, fraction glyphs), a handful of accented Latin-1 letters (`auml`,
`ouml`, `uuml` and capitals, `eacute`, `egrave`, `agrave`, `ccedil`,
`ntilde`, `szlig`), and a handful of common math/arrow symbols (`larr`,
`rarr`, `uarr`, `darr`, `harr`, `infin`, `ne`, `le`, `ge`, `permil`).

The exact list and values live in `crates/html/src/entities.rs`
(`ENTITY_TABLE`).

## Source and how it was fetched

- URL fetched: `https://html.spec.whatwg.org/entities.json`
- Fetched: 2026-08-09 (response `Date: Sun, 09 Aug 2026 07:55:31 GMT`)
- Response `Last-Modified: Wed, 12 Nov 2025 00:20:03 GMT`, `ETag:
  "6913d2b3-239e9"` — this pins the exact generated snapshot of the table
  that was read, independent of any single source-repo commit (see below).
- This file is a **build artifact** of the `whatwg/html` spec (generated
  from the spec's `entities.inc`/build tooling at publish time), not a
  version-controlled path in that repository directly — `entities.json` is
  not tracked at a stable path in `whatwg/html`, so there is no single
  vendor-able source commit for it the way there is for e.g. WPT test
  files. As an approximate anchor, `whatwg/html`'s `main` branch tip at
  fetch time was commit `24c5e48bf66ea61bc199ec6338c81258275ba9c6`.
- The 63 curated entries were extracted from the full downloaded
  `entities.json` with a one-off local Node.js script (not committed —
  see `crates/html/src/entities.rs` for the resulting static table); the
  numeric `codepoints` values transcribed into `ENTITY_TABLE` are the real
  values from that fetch, not hand-typed from memory. The `legacy` flag
  records whether the same fetch's data also contained a
  no-trailing-semicolon key (`"&name"` in addition to `"&name;"`).
- The full raw `entities.json` (145,897 bytes, ~2231 entries) was **not**
  committed to this repository — only the derived 63-entry subset above.

## License

Per `whatwg/html`'s `LICENSE`: the spec content is licensed
**CC BY 4.0**, and "to the extent portions of it are incorporated into
source code, such portions in the source code are licensed under the
**BSD 3-Clause License** instead." `ENTITY_TABLE` (name → code point data
incorporated into `crates/html/src/entities.rs`) falls under that
source-code carve-out. This is a permissive license consistent with
`OWNER_DECISIONS.md` D01-B's clean-room/permissive-license default; no
additional legal flag is required for this reduced subset (contrast with
the deferred full-table vendoring below, which should still get a
license/attribution pass at that time for completeness).

## What is deliberately deferred (tracked, not silently dropped)

- **The full ~2231-entry WHATWG named-character-reference table.** This
  MVP ships 63 of them. Any named entity outside this subset — the large
  majority of the real table, including most of the multi-character
  Unicode math/technical entities and the small number of two-code-point
  legacy entities — falls through to the tokenizer's "unknown named
  character reference" path (literal `&name;` text is emitted, with a
  `ParseErrorCode::UnknownNamedCharacterReference` diagnostic when the
  scanned name looked complete). This is a real, user-visible content gap
  versus a spec-conformant browser and must be closed before WPT
  conformance work can pass `namedEntities.test`/`entities.test` in full.
- **The full WPT/html5lib-tests tokenizer corpus** (`html5lib-tests/
  tokenizer/*.test`, design §6) — not vendored in this pass at all (no
  files under `tests/wpt/`). Tracked as a separate, explicitly deferred
  follow-on task; see `.agent-state/evidence/M2-T03.md`.
- **cargo-fuzz harness and curated corpus** (design §5) — not created in
  this pass. Tracked as a separate, explicitly deferred follow-on task; see
  `.agent-state/evidence/M2-T03.md`.

## Reversibility

If counsel review later prefers a different provenance path for the full
table, design §2 notes the alternative: regenerate independently from HTML
spec §13.5's normative listing (different provenance, same data). Nothing
in this MVP's 63-entry subset or its matching algorithm needs to change
either way — only `ENTITY_TABLE`'s size grows.
