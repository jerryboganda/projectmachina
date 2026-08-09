//! Non-fatal tokenizer diagnostics.
//!
//! Deliberately **not** the `ERROR_MODEL.md` canonical `{code,category,
//! retryable,...}` shape (see `architecture/ERROR_MODEL.md`) — that shape is
//! for command-bus/protocol-facing failures. `Diagnostic` is a local,
//! allocation-light, non-fatal event type correlated to a byte offset in the
//! input stream. A higher layer (native-core or the M2-T04 tree builder)
//! decides whether any of this becomes a canonical protocol error.

use crate::limits::LimitKind;

/// A parse error or limit event raised while tokenizing. Never fatal: the
/// tokenizer always keeps scanning after emitting one (see `src/tokenizer.rs`
/// for the specific recovery action taken for each case).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub code: ParseErrorCode,
    /// Stream-wide byte offset (across all `feed()` calls) at which the
    /// diagnostic was raised.
    pub position: u64,
    pub severity: Severity,
}

impl Diagnostic {
    pub(crate) fn new(code: ParseErrorCode, position: u64, severity: Severity) -> Self {
        Self {
            code,
            position,
            severity,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Warning,
    Error,
}

/// Parse error codes.
///
/// This is a **representative subset** of the ~60 named parse errors listed
/// in WHATWG HTML §13.2.2, covering every error path this tokenizer's
/// implemented state coverage can actually raise, plus Machina-specific
/// `LimitExceeded`/`TooManyDiagnostics` events. It is not a claim of full
/// spec parity — see `.agent-state/evidence/M2-T03.md` for the tracked
/// follow-up to broaden this to the full named-error list once WPT
/// conformance work is in scope.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseErrorCode {
    UnexpectedNullCharacter,
    UnexpectedQuestionMarkInsteadOfTagName,
    EofBeforeTagName,
    InvalidFirstCharacterOfTagName,
    MissingEndTagName,
    EofInTag,
    EofInComment,
    EofInDoctype,
    EofInCdata,
    EofInScriptHtmlCommentLikeText,
    AbruptClosingOfEmptyComment,
    IncorrectlyClosedComment,
    IncorrectlyOpenedComment,
    NestedComment,
    MissingWhitespaceBeforeDoctypeName,
    MissingDoctypeName,
    MissingWhitespaceAfterDoctypePublicKeyword,
    MissingWhitespaceAfterDoctypeSystemKeyword,
    MissingQuoteBeforeDoctypePublicIdentifier,
    MissingQuoteBeforeDoctypeSystemIdentifier,
    MissingDoctypePublicIdentifier,
    MissingDoctypeSystemIdentifier,
    MissingWhitespaceBetweenDoctypePublicAndSystemIdentifiers,
    UnexpectedCharacterAfterDoctypeSystemIdentifier,
    /// Machina-specific naming for WHATWG's "invalid-character-sequence-
    /// after-doctype-name" parse error (raised when the PUBLIC/SYSTEM
    /// keyword scan after a DOCTYPE name fails to match either keyword).
    InvalidCharacterSequenceAfterDoctypeName,
    AbruptDoctypePublicIdentifier,
    AbruptDoctypeSystemIdentifier,
    CdataInHtmlContent,
    CharacterReferenceOutsideUnicodeRange,
    ControlCharacterReference,
    NoncharacterCharacterReference,
    NullCharacterReference,
    SurrogateCharacterReference,
    MissingSemicolonAfterCharacterReference,
    UnknownNamedCharacterReference,
    AbsenceOfDigitsInNumericCharacterReference,
    MissingAttributeValue,
    UnexpectedCharacterInAttributeName,
    UnexpectedCharacterInUnquotedAttributeValue,
    UnexpectedEqualsSignBeforeAttributeName,
    UnexpectedSolidusInTag,
    DuplicateAttribute,
    EndTagWithAttributes,
    EndTagWithTrailingSolidus,
    /// Machina-specific: the byte stream contained a sequence that is not
    /// valid UTF-8. Recovery is always U+FFFD substitution — never a panic
    /// or a dropped byte range larger than the invalid sequence itself.
    InvalidUtf8Sequence,
    /// A tokenizer-internal bounded-input limit (see `TokenizerLimits`) was
    /// hit. The tokenizer always applies the documented recovery action
    /// (truncate/skip/resync) rather than aborting.
    LimitExceeded(LimitKind),
    /// More than `TokenizerLimits::max_diagnostics_buffered` diagnostics
    /// were raised within a single `feed()`/`finish()` call; the rest were
    /// coalesced into this single summary event.
    TooManyDiagnostics,
}
