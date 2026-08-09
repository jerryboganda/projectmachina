//! WHATWG HTML §13.2.4.1's 23 insertion modes, one flat enum, doc-commented
//! with the spec anchor each corresponds to. Dispatch is one non-recursive
//! `loop { match mode { ... } }` in `builder.rs` — "reprocess the token
//! under a different insertion mode" returns [`Dispatch::Reprocess`],
//! consumed by the outer loop and bounded by
//! `TreeBuilderLimits::max_reprocess_hops` (design §1, §7f), matching the
//! flat-FSM posture of `machina_html`'s tokenizer one layer down.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertionMode {
    /// §13.2.6.4.1
    Initial,
    /// §13.2.6.4.2
    BeforeHtml,
    /// §13.2.6.4.3
    BeforeHead,
    /// §13.2.6.4.4
    InHead,
    /// §13.2.6.4.5
    InHeadNoscript,
    /// §13.2.6.4.6
    AfterHead,
    /// §13.2.6.4.7
    InBody,
    /// §13.2.6.4.8 — generic RCDATA/RAWTEXT/script-data text mode, entered
    /// via the tokenizer text-content-state hooks (design §5).
    Text,
    /// §13.2.6.4.9
    InTable,
    /// §13.2.6.4.10
    InTableText,
    /// §13.2.6.4.11
    InCaption,
    /// §13.2.6.4.12
    InColumnGroup,
    /// §13.2.6.4.13
    InTableBody,
    /// §13.2.6.4.14
    InRow,
    /// §13.2.6.4.15
    InCell,
    /// §13.2.6.4.16
    InSelect,
    /// §13.2.6.4.17
    InSelectInTable,
    /// §13.2.6.4.18
    InTemplate,
    /// §13.2.6.4.19
    AfterBody,
    /// §13.2.6.4.20
    InFrameset,
    /// §13.2.6.4.21
    AfterFrameset,
    /// §13.2.6.4.22
    AfterAfterBody,
    /// §13.2.6.4.23
    AfterAfterFrameset,
}

/// What the outer dispatch loop should do after one insertion-mode handler
/// runs for the current token.
pub(crate) enum Dispatch {
    /// The token was fully consumed; move on to the next tokenizer event.
    Consumed,
    /// "Reprocess the token" under a (possibly different) insertion mode —
    /// consumed by the outer loop, bounded by `max_reprocess_hops`.
    Reprocess(InsertionMode),
}
