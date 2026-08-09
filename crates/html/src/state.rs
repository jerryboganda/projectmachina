//! Tokenizer state machine states (WHATWG HTML §13.2.5), grouped by concern.
//! A single flat enum, dispatched by one non-recursive `match` in
//! `src/tokenizer.rs` — see that module's top comment for why this makes
//! stack-overflow-via-adversarial-input structurally impossible at this
//! layer.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum State {
    // --- Text-content modes (§13.2.5.1-5), selected externally via
    // `Tokenizer::switch_to` (design §7). ---
    Data,
    Rcdata,
    Rawtext,
    ScriptData,
    Plaintext,

    // --- Tag open / name (§13.2.5.6-9,32-34) ---
    TagOpen,
    EndTagOpen,
    TagName,

    // --- RCDATA end-tag recognition (§13.2.5.10-13) ---
    RcdataLessThanSign,
    RcdataEndTagOpen,
    RcdataEndTagName,

    // --- RAWTEXT end-tag recognition (§13.2.5.14-17) ---
    RawtextLessThanSign,
    RawtextEndTagOpen,
    RawtextEndTagName,

    // --- Script data + the 12-state escaping subgroup (§13.2.5.18-33) ---
    ScriptDataLessThanSign,
    ScriptDataEndTagOpen,
    ScriptDataEndTagName,
    ScriptDataEscapeStart,
    ScriptDataEscapeStartDash,
    ScriptDataEscaped,
    ScriptDataEscapedDash,
    ScriptDataEscapedDashDash,
    ScriptDataEscapedLessThanSign,
    ScriptDataEscapedEndTagOpen,
    ScriptDataEscapedEndTagName,
    ScriptDataDoubleEscapeStart,
    ScriptDataDoubleEscaped,
    ScriptDataDoubleEscapedDash,
    ScriptDataDoubleEscapedDashDash,
    ScriptDataDoubleEscapedLessThanSign,
    ScriptDataDoubleEscapeEnd,

    // --- Attributes (§13.2.5.32-43); dedup-first-wins applied here ---
    BeforeAttributeName,
    AttributeName,
    AfterAttributeName,
    BeforeAttributeValue,
    AttributeValueDoubleQuoted,
    AttributeValueSingleQuoted,
    AttributeValueUnquoted,
    AfterAttributeValueQuoted,
    SelfClosingStartTag,

    // --- Comments + markup declaration (§13.2.5.41,44-52) ---
    BogusComment,
    MarkupDeclarationOpen,
    CommentStart,
    CommentStartDash,
    Comment,
    CommentLessThanSign,
    CommentLessThanSignBang,
    CommentLessThanSignBangDash,
    CommentLessThanSignBangDashDash,
    CommentEndDash,
    CommentEnd,
    CommentEndBang,

    // --- DOCTYPE (§13.2.5.53-68) ---
    Doctype,
    BeforeDoctypeName,
    DoctypeName,
    AfterDoctypeName,
    AfterDoctypePublicKeyword,
    BeforeDoctypePublicIdentifier,
    DoctypePublicIdentifierDoubleQuoted,
    DoctypePublicIdentifierSingleQuoted,
    AfterDoctypePublicIdentifier,
    BetweenDoctypePublicAndSystemIdentifiers,
    AfterDoctypeSystemKeyword,
    BeforeDoctypeSystemIdentifier,
    DoctypeSystemIdentifierDoubleQuoted,
    DoctypeSystemIdentifierSingleQuoted,
    AfterDoctypeSystemIdentifier,
    BogusDoctype,

    // --- CDATA (foreign-content only; §13.2.5.69-71) ---
    CdataSection,
    CdataSectionBracket,
    CdataSectionEnd,

    // --- Character references (§13.2.5.72-80) ---
    //
    // Note: WHATWG §13.2.5.74's "ambiguous ampersand state" is not a
    // separate variant here — this implementation resolves that behavior
    // inline inside `NamedCharacterReference`'s finalization (see
    // `src/tokenizer.rs`), since a failed match's fallback character-by-
    // character reconsumption is equivalent output-wise. Documented as a
    // deliberate simplification, not an omission.
    CharacterReference,
    NamedCharacterReference,
    NumericCharacterReference,
    HexadecimalCharacterReferenceStart,
    DecimalCharacterReferenceStart,
    HexadecimalCharacterReference,
    DecimalCharacterReference,
    NumericCharacterReferenceEnd,
}
