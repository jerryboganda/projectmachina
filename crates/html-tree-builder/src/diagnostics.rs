//! Non-fatal tree-construction diagnostics, mirroring `machina_html`'s
//! `Diagnostic` posture: collected, never fatal, never turned into a panic
//! or an `Err`. A higher layer decides whether any of this becomes a
//! canonical protocol error (see `crates/html/src/diagnostics.rs`'s doc
//! comment for the same reasoning one layer down).

/// A tree-construction-level diagnostic.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum Diagnostic {
    /// Forwarded verbatim from the tokenizer.
    Tokenizer(machina_html::Diagnostic),
    /// Design §7b: an adversarially deep run of unclosed same-tag (or
    /// otherwise unbounded) nesting hit
    /// [`crate::limits::TreeBuilderLimits::max_open_elements_depth`] before
    /// any DOM call was attempted. The offending start tag is treated as a
    /// failed open (never pushed, never inserted into the DOM); parsing
    /// continues under the current stack. Deliberately spec-deviating
    /// (WHATWG defines no numeric limit) but deterministic.
    NestingLimitExceeded { local_name: String, depth: usize },
    /// A start/end tag was ignored under the current insertion mode as a
    /// deliberate, spec-informed simplification (see
    /// `.agent-state/evidence/M2-T04.md` for the list of known gaps this
    /// covers, e.g. frameset documents).
    TokenIgnored {
        local_name: String,
        mode: &'static str,
    },
    /// A generic parse-error-equivalent condition raised directly by the
    /// tree construction algorithm (misnesting, unexpected end tag, stray
    /// end tag with no matching open element, etc).
    ParseError { detail: String },
}
