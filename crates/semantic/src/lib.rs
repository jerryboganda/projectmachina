//! `machina-semantic` — basic semantic role/accessible-name derivation,
//! bounded markdown generation, and metadata/schema extraction on top of
//! Project Machina's native DOM (`machina-dom`) and query engine
//! (`machina-selectors`).
//!
//! Implements M2-T13's deliverables: "Derive basic roles, accessible names,
//! interactive elements, headings, links and forms. Generate bounded
//! markdown and metadata/schema extraction from the live DOM. Attach
//! document/semantic revisions and stable handles."
//!
//! See `.agent-state/evidence/M2-T13.md` for the full design rationale (this
//! task had no pre-written design doc, unlike M2-T02 through M2-T11) —
//! exactly what subset of ARIA/accname/CommonMark/structured-data this crate
//! implements versus disclosed, deferred gaps.
//!
//! # Crate identity
//!
//! Depends only on `machina-dom` (arena/handles/revision) and
//! `machina-selectors` (the compiled-selector query engine, used to locate
//! role-candidate elements instead of a bespoke tree-walk-and-match). No
//! protocol/command-model/command-bus/capability crate dependency, matching
//! the sibling `crates/html-tree-builder`/`crates/selectors`
//! dependency-direction convention (see
//! `.agent-state/design/M2-M1-contract-compatibility-checklist.md`).
//!
//! # No `unsafe`, no `unwrap`/`expect` on any DOM-content-reachable path
//!
//! Page content is external/adversarial input in this repo's threat model
//! (the same bar the HTML tokenizer/tree builder and `crates/selectors`
//! hold themselves to) — every DOM-content-reachable path in this crate
//! returns a typed [`SemanticError`] instead of panicking.
//!
//! # No duplicate DOM copies
//!
//! Every extraction function reads directly from the live [`machina_dom::
//! Document`]/[`machina_dom::ElementHandle`]s it is given: no `clone_node`
//! call and no parallel tree/node-list structure is ever built anywhere in
//! this crate — only small, owned output types (`Vec<ElementHandle>`,
//! `HashMap<String, ElementHandle>`, output `String`s) are allocated. See
//! `.agent-state/evidence/M2-T13.md`'s "no duplicate DOM copies" section for
//! the code-review-based verification of this claim (`grep -rn clone_node
//! crates/semantic/src/` finds nothing).
//!
//! # Revisions and stable handles
//!
//! Every top-level result ([`SemanticIndex`], [`MarkdownDocument`],
//! [`DocumentMetadata`]) self-stamps with the [`machina_dom::Revision`] it
//! was computed against, exactly mirroring `machina_selectors::QueryResult`
//! /`XPathResult`'s contract: staleness detection is plain `result.revision
//! != document.revision()`, never guessed or implicit. Every extracted item
//! (heading, link, form, interactive element) carries the
//! [`machina_dom::ElementHandle`] it was derived from, so a caller can act
//! on it directly. This crate holds no internal cache — every call is a
//! fresh computation against the `Document` passed in, matching
//! `crates/selectors`'s own "thin layer on top" no-indexing-MVP choice.

#![forbid(unsafe_code)]

mod error;
mod extract;
mod limits;
mod markdown;
mod metadata;
mod role;
mod text;
mod walk;

pub use error::SemanticError;
pub use extract::{extract_semantic_index, FormNode, LinkNode, SemanticIndex, SemanticNode};
pub use limits::LimitKind;
pub use markdown::{generate_markdown, MarkdownDocument};
pub use metadata::{extract_metadata, DocumentMetadata};
