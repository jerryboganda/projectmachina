//! `machina-html-tree-builder` — HTML tree construction (WHATWG HTML
//! §13.2.6) on top of `machina-html`'s streaming tokenizer (M2-T03) and
//! `machina-dom`'s arena-backed `Document` (M2-T05).
//!
//! See `.agent-state/design/M2-T04-tree-builder-design.md` for the full
//! design this crate implements, and `.agent-state/evidence/M2-T04.md` for
//! exactly what shipped versus what is deferred (most notably: full
//! WPT/html5lib-tests conformance, which has no vendoring/harness
//! infrastructure anywhere in this repository yet).
//!
//! # Crate identity
//!
//! A sibling of `crates/html` and `crates/dom`, not a module inside
//! either — `crates/html` is a deliberately zero-runtime-dependency crate
//! and must not gain a dependency on `crates/dom`; this crate is where
//! that dependency (on both) belongs. See design doc §0.
//!
//! # No `unsafe`, no panics on parser-input-reachable paths
//!
//! Malformed HTML never produces an `Err` — only a collected
//! [`Diagnostic`]; adversarial nesting fails closed deterministically
//! (design §7) well before `machina_dom`'s own hierarchy/depth guards
//! could ever fire.
#![forbid(unsafe_code)]

mod active_formatting;
mod adoption_agency;
mod builder;
mod checkpoint;
mod diagnostics;
mod error;
mod foreign;
mod foster_parent;
mod limits;
mod modes;
mod open_elements;
mod rules;
mod special;

pub use builder::TreeBuilder;
pub use checkpoint::{ScriptCheckpoint, ScriptSource, TreeBuilderOutcome};
pub use diagnostics::Diagnostic;
pub use error::TreeBuilderError;
pub use limits::TreeBuilderLimits;
pub use modes::InsertionMode;

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send<T: Send>() {}

    #[test]
    fn tree_builder_is_send() {
        assert_send::<TreeBuilder>();
    }
}
