//! Public token shapes (design §7), consumed by the M2-T04 tree builder.
//!
//! `String`/`Vec` are deliberately used for MVP correctness-first; small-
//! string/interning is a tracked non-blocking perf follow-up (interning
//! itself is deferred to `machina-dom`, M2-T05).

/// One tokenizer output event's payload.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Doctype(DoctypeToken),
    StartTag(TagToken),
    EndTag(TagToken),
    Comment(CommentToken),
    /// A run of character data. Consecutive `Character` tokens may occur —
    /// see the streaming equivalence contract in `src/tokenizer.rs` for
    /// exactly when a run is flushed into its own token.
    Character(CharacterToken),
    Eof,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TagToken {
    pub name: String,
    pub self_closing: bool,
    /// Deduplicated, first-occurrence-wins, in original encounter order.
    pub attributes: Vec<Attribute>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attribute {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DoctypeToken {
    pub name: Option<String>,
    pub public_id: Option<String>,
    pub system_id: Option<String>,
    pub force_quirks: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommentToken {
    pub data: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CharacterToken {
    pub data: String,
}

/// A pulled tokenizer output: either a token or an interleaved diagnostic
/// (interleaved so error positions correlate with the token being built —
/// design §7).
#[derive(Clone, Debug, PartialEq)]
pub enum TokenizerEvent {
    Token(Token),
    Diagnostic(crate::diagnostics::Diagnostic),
}

/// The text-content mode the tokenizer scans in. Selected externally by the
/// tree builder immediately after receiving a `StartTag` token for an
/// element whose content model requires it (`<title>`/`<textarea>` →
/// `Rcdata`, `<style>`/`<xmp>`/... → `Rawtext`, `<script>` → `ScriptData`,
/// `<plaintext>` → `Plaintext`) — see design §7's external state-switch
/// hook.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TextContentState {
    #[default]
    Data,
    Rcdata,
    Rawtext,
    ScriptData,
    Plaintext,
}
