//! Hand-written tokenizer fixtures (acceptance criterion: "priority
//! tokenizer fixtures pass" — see `.agent-state/evidence/M2-T03.md` for the
//! WPT-vendoring caveat this substitutes for in this pass). Covers tag/
//! attribute parsing, comments, DOCTYPE, RCDATA/RAWTEXT/script-data (incl.
//! the escaping subgroup), named + numeric character references, and CDATA.

mod common;

use common::tokenize;
use machina_html::{
    Attribute, CharacterToken, CommentToken, DoctypeToken, ParseErrorCode, TagToken,
    TextContentState, Token, Tokenizer, TokenizerLimits,
};

fn attr(name: &str, value: &str) -> Attribute {
    Attribute {
        name: name.to_owned(),
        value: value.to_owned(),
    }
}

fn text(data: &str) -> Token {
    Token::Character(CharacterToken {
        data: data.to_owned(),
    })
}

#[test]
fn simple_element_with_attributes_and_text() {
    let (tokens, diags) = tokenize(
        br#"<div class="a" id='b' disabled>text</div>"#,
        TokenizerLimits::default(),
    );
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(
        tokens,
        vec![
            Token::StartTag(TagToken {
                name: "div".into(),
                self_closing: false,
                attributes: vec![attr("class", "a"), attr("id", "b"), attr("disabled", "")],
            }),
            text("text"),
            Token::EndTag(TagToken {
                name: "div".into(),
                self_closing: false,
                attributes: vec![],
            }),
            Token::Eof,
        ]
    );
}

#[test]
fn self_closing_tag() {
    let (tokens, diags) = tokenize(b"<br/>", TokenizerLimits::default());
    assert!(diags.is_empty());
    assert_eq!(
        tokens,
        vec![
            Token::StartTag(TagToken {
                name: "br".into(),
                self_closing: true,
                attributes: vec![],
            }),
            Token::Eof,
        ]
    );
}

#[test]
fn tag_names_and_attribute_names_are_lowercased() {
    let (tokens, _) = tokenize(br#"<DIV ID="x"></DIV>"#, TokenizerLimits::default());
    assert_eq!(
        tokens[0],
        Token::StartTag(TagToken {
            name: "div".into(),
            self_closing: false,
            attributes: vec![attr("id", "x")],
        })
    );
}

#[test]
fn duplicate_attribute_is_discarded_first_wins() {
    let (tokens, diags) = tokenize(
        br#"<a href="first" href="second">"#,
        TokenizerLimits::default(),
    );
    assert_eq!(
        tokens[0],
        Token::StartTag(TagToken {
            name: "a".into(),
            self_closing: false,
            attributes: vec![attr("href", "first")],
        })
    );
    assert!(diags
        .iter()
        .any(|d| d.code == ParseErrorCode::DuplicateAttribute));
}

#[test]
fn comment() {
    let (tokens, diags) = tokenize(b"<!-- hello -->", TokenizerLimits::default());
    assert!(diags.is_empty());
    assert_eq!(
        tokens,
        vec![
            Token::Comment(CommentToken {
                data: " hello ".into()
            }),
            Token::Eof,
        ]
    );
}

#[test]
fn abrupt_closing_of_empty_comment() {
    let (tokens, diags) = tokenize(b"<!-->", TokenizerLimits::default());
    assert_eq!(
        tokens,
        vec![
            Token::Comment(CommentToken {
                data: String::new()
            }),
            Token::Eof
        ]
    );
    assert!(diags
        .iter()
        .any(|d| d.code == ParseErrorCode::AbruptClosingOfEmptyComment));
}

#[test]
fn bogus_comment_from_unrecognized_bang() {
    let (tokens, diags) = tokenize(b"<!wrong>", TokenizerLimits::default());
    assert_eq!(
        tokens,
        vec![
            Token::Comment(CommentToken {
                data: "wrong".into()
            }),
            Token::Eof
        ]
    );
    assert!(diags
        .iter()
        .any(|d| d.code == ParseErrorCode::IncorrectlyOpenedComment));
}

#[test]
fn nested_comment_dash_markers_inside_a_comment() {
    let (tokens, _) = tokenize(b"<!-- a <!-- b --> c -->", TokenizerLimits::default());
    // The comment ends at the first `-->`; trailing " c -->" becomes
    // literal text/a stray `>` per the bogus-comment-adjacent behavior of
    // the real spec's comment grammar (comments cannot be nested).
    assert_eq!(
        tokens[0],
        Token::Comment(CommentToken {
            data: " a <!-- b ".into()
        })
    );
}

#[test]
fn doctype_simple() {
    let (tokens, diags) = tokenize(b"<!DOCTYPE html>", TokenizerLimits::default());
    assert!(diags.is_empty());
    assert_eq!(
        tokens,
        vec![
            Token::Doctype(DoctypeToken {
                name: Some("html".into()),
                public_id: None,
                system_id: None,
                force_quirks: false,
            }),
            Token::Eof,
        ]
    );
}

#[test]
fn doctype_case_insensitive_keyword() {
    let (tokens, _) = tokenize(b"<!doctype HTML>", TokenizerLimits::default());
    assert_eq!(
        tokens[0],
        Token::Doctype(DoctypeToken {
            name: Some("html".into()),
            public_id: None,
            system_id: None,
            force_quirks: false,
        })
    );
}

#[test]
fn doctype_with_public_and_system_identifiers() {
    let input =
        br#"<!DOCTYPE html PUBLIC "-//W3C//DTD HTML 4.01//EN" "http://www.w3.org/TR/html4/strict.dtd">"#;
    let (tokens, diags) = tokenize(input, TokenizerLimits::default());
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(
        tokens[0],
        Token::Doctype(DoctypeToken {
            name: Some("html".into()),
            public_id: Some("-//W3C//DTD HTML 4.01//EN".into()),
            system_id: Some("http://www.w3.org/TR/html4/strict.dtd".into()),
            force_quirks: false,
        })
    );
}

#[test]
fn doctype_missing_name_forces_quirks() {
    let (tokens, diags) = tokenize(b"<!DOCTYPE >", TokenizerLimits::default());
    assert_eq!(
        tokens[0],
        Token::Doctype(DoctypeToken {
            name: None,
            public_id: None,
            system_id: None,
            force_quirks: true,
        })
    );
    assert!(diags
        .iter()
        .any(|d| d.code == ParseErrorCode::MissingDoctypeName));
}

fn tokenize_with_switch(
    input: &[u8],
    after_start_tag: &str,
    mode: TextContentState,
) -> (Vec<Token>, Vec<machina_html::Diagnostic>) {
    let mut tokenizer = Tokenizer::new(TokenizerLimits::default());
    tokenizer.feed(input);
    tokenizer.finish();
    let mut tokens = Vec::new();
    let mut diags = Vec::new();
    while let Some(event) = tokenizer.next_event() {
        match event {
            machina_html::TokenizerEvent::Token(Token::StartTag(tag))
                if tag.name == after_start_tag =>
            {
                tokens.push(Token::StartTag(tag));
                tokenizer.switch_to(mode);
            }
            machina_html::TokenizerEvent::Token(t) => tokens.push(t),
            machina_html::TokenizerEvent::Diagnostic(d) => diags.push(d),
        }
    }
    (tokens, diags)
}

#[test]
fn rcdata_decodes_entities_but_not_tags() {
    let (tokens, diags) = tokenize_with_switch(
        b"<title>a &lt; b</title>",
        "title",
        TextContentState::Rcdata,
    );
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(
        tokens,
        vec![
            Token::StartTag(TagToken {
                name: "title".into(),
                self_closing: false,
                attributes: vec![],
            }),
            text("a < b"),
            Token::EndTag(TagToken {
                name: "title".into(),
                self_closing: false,
                attributes: vec![],
            }),
            Token::Eof,
        ]
    );
}

#[test]
fn rawtext_does_not_decode_entities() {
    let (tokens, diags) = tokenize_with_switch(
        b"<style>a { color: &notarealthing; }</style>",
        "style",
        TextContentState::Rawtext,
    );
    assert!(
        diags.is_empty(),
        "rawtext must not attempt entity decoding: {diags:?}"
    );
    assert_eq!(tokens[1], text("a { color: &notarealthing; }"));
}

#[test]
fn rawtext_end_tag_name_mismatch_is_literal_text() {
    // "</style" doesn't match the last start tag ("div"), so it must be
    // reconstructed as literal text, not treated as a real end tag.
    let (tokens, _) =
        tokenize_with_switch(b"<div></style></div>", "div", TextContentState::Rawtext);
    assert_eq!(tokens[1], text("</style>"));
    assert_eq!(
        tokens[2],
        Token::EndTag(TagToken {
            name: "div".into(),
            self_closing: false,
            attributes: vec![],
        })
    );
}

#[test]
fn script_data_does_not_interpret_nested_tag_like_text() {
    let (tokens, diags) = tokenize_with_switch(
        br#"<script>document.write("<div>")</script>"#,
        "script",
        TextContentState::ScriptData,
    );
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(tokens[1], text(r#"document.write("<div>")"#));
    assert_eq!(
        tokens[2],
        Token::EndTag(TagToken {
            name: "script".into(),
            self_closing: false,
            attributes: vec![],
        })
    );
}

#[test]
fn script_data_escaped_comment_round_trips_literally() {
    let (tokens, diags) = tokenize_with_switch(
        b"<script><!--x--></script>",
        "script",
        TextContentState::ScriptData,
    );
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(tokens[1], text("<!--x-->"));
    assert_eq!(
        tokens[2],
        Token::EndTag(TagToken {
            name: "script".into(),
            self_closing: false,
            attributes: vec![],
        })
    );
}

#[test]
fn script_data_double_escape_keeps_nested_script_tags_literal() {
    // Inside an escaped comment, a literal `<script>`/`</script>` pair
    // toggles double-escaped mode and stays literal text throughout; only
    // the final, unmatched `</script>` (outside any `<!--`) really closes
    // the element.
    let input = b"<script><!--<script>inner</script>--></script>";
    let (tokens, diags) = tokenize_with_switch(input, "script", TextContentState::ScriptData);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(tokens[1], text("<!--<script>inner</script>-->"));
    assert_eq!(
        tokens[2],
        Token::EndTag(TagToken {
            name: "script".into(),
            self_closing: false,
            attributes: vec![],
        })
    );
}

#[test]
fn numeric_character_references_decimal_and_hex() {
    let (tokens, diags) = tokenize(b"<p>&#65;&#x42;</p>", TokenizerLimits::default());
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(tokens[1], text("AB"));
}

#[test]
fn numeric_character_reference_null_becomes_replacement_with_diagnostic() {
    let (tokens, diags) = tokenize(b"<p>&#0;</p>", TokenizerLimits::default());
    assert_eq!(tokens[1], text("\u{FFFD}"));
    assert!(diags
        .iter()
        .any(|d| d.code == ParseErrorCode::NullCharacterReference));
}

#[test]
fn numeric_character_reference_c1_control_remapping() {
    // 0x80 remaps to U+20AC (EURO SIGN) per the fixed C1 table.
    let (tokens, _) = tokenize(b"<p>&#128;</p>", TokenizerLimits::default());
    assert_eq!(tokens[1], text("\u{20AC}"));
}

#[test]
fn named_character_reference_with_semicolon() {
    let (tokens, diags) = tokenize(b"<p>&amp;&hellip;</p>", TokenizerLimits::default());
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(tokens[1], text("&\u{2026}"));
}

#[test]
fn named_character_reference_legacy_without_semicolon() {
    let (tokens, diags) = tokenize(b"<p>&amp is cool</p>", TokenizerLimits::default());
    assert_eq!(tokens[1], text("& is cool"));
    assert!(
        diags.is_empty(),
        "legacy no-semicolon entities must not raise a diagnostic"
    );
}

#[test]
fn named_character_reference_non_legacy_without_semicolon_still_substitutes_with_diagnostic() {
    let (tokens, diags) = tokenize(b"<p>&apos test</p>", TokenizerLimits::default());
    assert_eq!(tokens[1], text("' test"));
    assert!(diags
        .iter()
        .any(|d| d.code == ParseErrorCode::MissingSemicolonAfterCharacterReference));
}

#[test]
fn unknown_named_character_reference_stays_literal_with_diagnostic() {
    let (tokens, diags) = tokenize(b"<p>&am;</p>", TokenizerLimits::default());
    assert_eq!(tokens[1], text("&am;"));
    assert!(diags
        .iter()
        .any(|d| d.code == ParseErrorCode::UnknownNamedCharacterReference));
}

#[test]
fn bare_ampersand_is_literal() {
    let (tokens, diags) = tokenize(b"<p>Tom & Jerry</p>", TokenizerLimits::default());
    assert!(diags.is_empty());
    assert_eq!(tokens[1], text("Tom & Jerry"));
}

#[test]
fn character_reference_in_attribute_value() {
    let (tokens, _) = tokenize(br#"<a href="?a=1&amp;b=2">"#, TokenizerLimits::default());
    assert_eq!(
        tokens[0],
        Token::StartTag(TagToken {
            name: "a".into(),
            self_closing: false,
            attributes: vec![attr("href", "?a=1&b=2")],
        })
    );
}

#[test]
fn cdata_section_is_literal_text() {
    let (tokens, diags) = tokenize(b"<![CDATA[a<b]]>", TokenizerLimits::default());
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(tokens, vec![text("a<b"), Token::Eof]);
}

#[test]
fn null_byte_in_data_becomes_replacement_character_with_diagnostic() {
    let (tokens, diags) = tokenize(b"a\0b", TokenizerLimits::default());
    assert_eq!(tokens, vec![text("a\u{FFFD}b"), Token::Eof]);
    assert!(diags
        .iter()
        .any(|d| d.code == ParseErrorCode::UnexpectedNullCharacter));
}

#[test]
fn eof_in_unterminated_tag_abandons_the_tag() {
    let (tokens, diags) = tokenize(b"<div class=\"x", TokenizerLimits::default());
    // No StartTag is emitted for an abandoned tag -- only the Eof token.
    assert_eq!(tokens, vec![Token::Eof]);
    assert!(diags.iter().any(|d| d.code == ParseErrorCode::EofInTag));
}

#[test]
fn eof_in_unterminated_comment_still_emits_the_comment() {
    let (tokens, diags) = tokenize(b"<!-- never closed", TokenizerLimits::default());
    assert_eq!(
        tokens,
        vec![
            Token::Comment(CommentToken {
                data: " never closed".into()
            }),
            Token::Eof
        ]
    );
    assert!(diags.iter().any(|d| d.code == ParseErrorCode::EofInComment));
}
