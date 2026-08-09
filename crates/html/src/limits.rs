//! Bounded-input limits (design §4).
//!
//! The tokenizer's state machine is a flat, non-recursive driver (see
//! `src/tokenizer.rs`), so stack-overflow-via-adversarial-nesting is
//! structurally impossible at this layer. Every remaining unbounded-growth
//! surface (an attacker feeding an arbitrarily long tag name, attribute
//! value, comment, doctype field, character run, or diagnostics stream) is
//! covered by an explicit, configurable limit here, each with a defined,
//! non-aborting recovery action carried out in `src/tokenizer.rs`.

/// Which bounded-input limit a `ParseErrorCode::LimitExceeded` diagnostic
/// refers to.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LimitKind {
    TagNameLen,
    AttributeCount,
    AttributeValueLen,
    CommentLen,
    DoctypeFieldLen,
    CharacterRunLen,
    TagOrCommentByteSpan,
    NamedCharRefScanSteps,
}

/// Configurable bounds on adversarial input growth. Defaults match design
/// §4's table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TokenizerLimits {
    /// Truncate; keep scanning for the tag's terminating `>`.
    pub max_tag_name_len: usize,
    /// Stop adding new attributes; still parse/discard the remainder so the
    /// scan does not desynchronize from the byte stream.
    pub max_attribute_count: usize,
    /// Truncate (generous — legitimate `data:` URIs can be large).
    pub max_attribute_value_len: usize,
    /// Truncate; keep scanning for `-->` (bounds the classic unterminated
    /// `<!--` denial-of-service).
    pub max_comment_len: usize,
    /// Truncate a DOCTYPE name/public-id/system-id field.
    pub max_doctype_field_len: usize,
    /// Forced flush of the in-progress character run. This is the one
    /// legitimate, chunk-boundary-independent source of `Character` token
    /// *count* divergence sanctioned by the streaming equivalence contract
    /// (design §3) — set high enough that no realistic fixture triggers it
    /// under normal chunking.
    pub max_character_run_len: usize,
    /// Hard backstop distinct from the per-field limits above: catches
    /// constructs where no single field limit is exceeded but the overall
    /// tag/comment span is still unbounded (e.g. huge attribute *count*,
    /// each individually under `max_attribute_count`... actually bounded by
    /// it, but this also catches pathological attribute-name/value
    /// interleavings). Force-terminates the construct as bogus/oversized and
    /// resyncs to the data state.
    pub max_tag_or_comment_byte_span: usize,
    /// Defense-in-depth cap on how many characters the named-character-
    /// reference matcher will scan before giving up, independent of the
    /// table's own longest name.
    pub max_named_char_ref_scan_steps: usize,
    /// Diagnostics raised within a single `feed()`/`finish()` call beyond
    /// this count are coalesced into one `TooManyDiagnostics` event instead
    /// of growing an unbounded `Vec`.
    pub max_diagnostics_buffered: usize,
}

impl Default for TokenizerLimits {
    fn default() -> Self {
        Self {
            max_tag_name_len: 4 * 1024,
            max_attribute_count: 512,
            max_attribute_value_len: 8 * 1024 * 1024,
            max_comment_len: 16 * 1024 * 1024,
            max_doctype_field_len: 64 * 1024,
            max_character_run_len: 64 * 1024,
            max_tag_or_comment_byte_span: 8 * 1024 * 1024,
            max_named_char_ref_scan_steps: 64,
            max_diagnostics_buffered: 4096,
        }
    }
}
