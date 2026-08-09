//! Acceptance criterion: "adversarial input remains bounded and crash free"
//! (design §4's `TokenizerLimits`). Each test uses a deliberately small
//! limit so the adversarial case is reachable without generating megabytes
//! of test data, and asserts: (a) the tokenizer neither panics nor hangs,
//! (b) the documented recovery action (truncate/skip/resync) is what
//! actually happens, (c) the corresponding `LimitExceeded` diagnostic is
//! raised, and (d) the tokenizer keeps making forward progress afterward
//! (well-formed content after the adversarial construct still parses).

mod common;

use common::tokenize;
use machina_html::{LimitKind, ParseErrorCode, Token, TokenizerLimits};

fn has_limit_diagnostic(diags: &[machina_html::Diagnostic], kind: LimitKind) -> bool {
    diags
        .iter()
        .any(|d| matches!(d.code, ParseErrorCode::LimitExceeded(k) if k == kind))
}

#[test]
fn oversized_tag_name_is_truncated_not_unbounded() {
    let limits = TokenizerLimits {
        max_tag_name_len: 8,
        ..TokenizerLimits::default()
    };
    let name = "a".repeat(1000);
    let doc = format!("<{name}>after");
    let (tokens, diags) = tokenize(doc.as_bytes(), limits);
    assert!(has_limit_diagnostic(&diags, LimitKind::TagNameLen));
    match &tokens[0] {
        Token::StartTag(tag) => assert!(tag.name.len() <= limits.max_tag_name_len),
        other => panic!("expected a start tag, got {other:?}"),
    }
    // Forward progress: the tokenizer resynced and kept parsing.
    assert!(tokens
        .iter()
        .any(|t| matches!(t, Token::Character(c) if c.data == "after")));
}

#[test]
fn oversized_attribute_count_is_capped_not_unbounded() {
    let limits = TokenizerLimits {
        max_attribute_count: 4,
        ..TokenizerLimits::default()
    };
    let mut doc = String::from("<div");
    for i in 0..2000 {
        doc.push_str(&format!(" a{i}=\"{i}\""));
    }
    doc.push('>');
    let (tokens, diags) = tokenize(doc.as_bytes(), limits);
    assert!(has_limit_diagnostic(&diags, LimitKind::AttributeCount));
    match &tokens[0] {
        Token::StartTag(tag) => assert!(tag.attributes.len() <= limits.max_attribute_count),
        other => panic!("expected a start tag, got {other:?}"),
    }
}

#[test]
fn oversized_attribute_value_is_truncated_not_unbounded() {
    let limits = TokenizerLimits {
        max_attribute_value_len: 16,
        ..TokenizerLimits::default()
    };
    let value = "x".repeat(100_000);
    let doc = format!("<a href=\"{value}\">after");
    let (tokens, diags) = tokenize(doc.as_bytes(), limits);
    assert!(has_limit_diagnostic(&diags, LimitKind::AttributeValueLen));
    match &tokens[0] {
        Token::StartTag(tag) => {
            assert!(tag.attributes[0].value.len() <= limits.max_attribute_value_len)
        }
        other => panic!("expected a start tag, got {other:?}"),
    }
    assert!(tokens
        .iter()
        .any(|t| matches!(t, Token::Character(c) if c.data == "after")));
}

#[test]
fn unterminated_giant_comment_is_bounded_and_terminates_at_eof() {
    let limits = TokenizerLimits {
        max_comment_len: 32,
        ..TokenizerLimits::default()
    };
    let filler = "z".repeat(200_000);
    let doc = format!("<!--{filler}"); // never closed
    let (tokens, diags) = tokenize(doc.as_bytes(), limits);
    assert!(has_limit_diagnostic(&diags, LimitKind::CommentLen));
    match &tokens[0] {
        Token::Comment(c) => assert!(c.data.len() <= limits.max_comment_len),
        other => panic!("expected a comment, got {other:?}"),
    }
    // Must still terminate (finish() was called by `tokenize`) rather than
    // hang waiting for a `-->` that never comes.
    assert!(matches!(tokens.last(), Some(Token::Eof)));
}

#[test]
fn oversized_doctype_field_is_truncated_not_unbounded() {
    let limits = TokenizerLimits {
        max_doctype_field_len: 8,
        ..TokenizerLimits::default()
    };
    let filler = "q".repeat(10_000);
    let doc = format!("<!DOCTYPE html PUBLIC \"{filler}\" \"{filler}\">");
    let (tokens, diags) = tokenize(doc.as_bytes(), limits);
    assert!(has_limit_diagnostic(&diags, LimitKind::DoctypeFieldLen));
    match &tokens[0] {
        Token::Doctype(d) => {
            let public_len = d.public_id.as_ref().map(String::len).unwrap_or(0);
            assert!(public_len <= limits.max_doctype_field_len);
        }
        other => panic!("expected a doctype, got {other:?}"),
    }
}

#[test]
fn character_run_limit_forces_flush_and_still_terminates() {
    let limits = TokenizerLimits {
        max_character_run_len: 4,
        ..TokenizerLimits::default()
    };
    let filler = "a".repeat(1000);
    let doc = format!("<p>{filler}</p>");
    let (tokens, diags) = tokenize(doc.as_bytes(), limits);
    assert!(has_limit_diagnostic(&diags, LimitKind::CharacterRunLen));
    let character_token_count = tokens
        .iter()
        .filter(|t| matches!(t, Token::Character(_)))
        .count();
    assert!(
        character_token_count > 1,
        "the run must have been split into multiple tokens"
    );
    let concatenated: String = tokens
        .iter()
        .filter_map(|t| match t {
            Token::Character(c) => Some(c.data.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        concatenated, filler,
        "content must still be reconstructed exactly"
    );
}

#[test]
fn tag_or_comment_byte_span_backstop_force_terminates_and_resyncs() {
    // Set every per-field limit generously high so only the aggregate span
    // backstop can fire, but keep the backstop itself small.
    let limits = TokenizerLimits {
        max_attribute_count: 100_000,
        max_attribute_value_len: 100_000,
        max_tag_or_comment_byte_span: 256,
        ..TokenizerLimits::default()
    };

    let mut doc = String::from("<div");
    for i in 0..500 {
        doc.push_str(&format!(" a{i}=\"v{i}\""));
    }
    doc.push('>');
    doc.push_str("<p>after</p>");

    let (tokens, diags) = tokenize(doc.as_bytes(), limits);
    assert!(
        has_limit_diagnostic(&diags, LimitKind::TagOrCommentByteSpan),
        "diagnostics: {diags:?}"
    );
    // The tokenizer must have resynced to data and gone on to parse the
    // well-formed `<p>after</p>` that follows -- forward progress, not a
    // hang or a stuck state.
    assert!(tokens
        .iter()
        .any(|t| matches!(t, Token::StartTag(tag) if tag.name == "p")));
    assert!(tokens
        .iter()
        .any(|t| matches!(t, Token::Character(c) if c.data == "after")));
}

#[test]
fn named_char_ref_scan_step_limit_is_bounded_and_falls_back_to_literal() {
    let limits = TokenizerLimits {
        max_named_char_ref_scan_steps: 2,
        ..TokenizerLimits::default()
    };
    // "middot" is 6 characters, all of which extend a valid table prefix,
    // so with a step budget of 2 the scan must abort before completing the
    // match.
    let (tokens, diags) = tokenize(b"<p>&middot;</p>", limits);
    assert!(has_limit_diagnostic(
        &diags,
        LimitKind::NamedCharRefScanSteps
    ));
    // No panic, no hang, and the reconstructed text still contains every
    // original byte (nothing silently dropped).
    let concatenated: String = tokens
        .iter()
        .filter_map(|t| match t {
            Token::Character(c) => Some(c.data.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(concatenated, "&middot;");
}

#[test]
fn diagnostics_are_coalesced_past_the_configured_cap() {
    let limits = TokenizerLimits {
        max_diagnostics_buffered: 5,
        ..TokenizerLimits::default()
    };
    // Every NUL byte raises an UnexpectedNullCharacter diagnostic.
    let doc = "\0".repeat(1000);
    let (_, diags) = tokenize(doc.as_bytes(), limits);
    assert!(
        diags.len() <= limits.max_diagnostics_buffered + 1,
        "diagnostics must be coalesced, got {} for cap {}",
        diags.len(),
        limits.max_diagnostics_buffered
    );
    assert!(diags
        .iter()
        .any(|d| d.code == ParseErrorCode::TooManyDiagnostics));
}

#[test]
fn deeply_nested_bogus_comment_openers_do_not_panic_or_hang() {
    let doc = "<!--".repeat(10_000) + &"-->".repeat(10_000);
    let (tokens, _diags) = tokenize(doc.as_bytes(), TokenizerLimits::default());
    assert!(matches!(tokens.last(), Some(Token::Eof)));
}

#[test]
fn invalid_utf8_and_null_bytes_in_every_major_state_do_not_panic() {
    let adversarial_inputs: Vec<Vec<u8>> = vec![
        b"<div\0 class=\"a\0b\"\0>\0text\0</div\0>".to_vec(),
        b"<!--\0-->".to_vec(),
        b"<!DOCTYPE\0 html\0>".to_vec(),
        vec![b'<', b'a', b'&', 0xFF, b'>'],
        vec![0xC0, 0x80, b'<', b'p', b'>'], // overlong-looking invalid bytes
        b"<script>\0</script>".to_vec(),
        b"<title>&\0;</title>".to_vec(),
        b"<![CDATA[\0]]>".to_vec(),
    ];
    for input in adversarial_inputs {
        let (tokens, _diags) = tokenize(&input, TokenizerLimits::default());
        assert!(
            matches!(tokens.last(), Some(Token::Eof)),
            "must terminate cleanly for input {input:?}"
        );
    }
}
