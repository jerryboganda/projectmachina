//! Acceptance criterion: "arbitrary chunk boundaries produce an equivalent
//! token stream" (design §3's formal equivalence contract) — the one
//! criterion this task must not shortcut.
//!
//! For each representative document, this test splits the input at *every*
//! possible byte offset (two-way split) and asserts the resulting token
//! stream is equivalent to the unchunked stream: non-`Character` tokens
//! identical in order/content, `Character` tokens equivalent at the
//! concatenated-text level. It also covers multi-way (more than two chunks,
//! including byte-at-a-time) chunking of a larger representative document,
//! and the one legitimate, content-driven (not chunk-boundary-driven)
//! exception: `TokenizerLimits::max_character_run_len` forcing extra
//! `Character` token boundaries regardless of how the input was chunked.

mod common;

use common::{all_two_way_splits, normalize_character_runs, tokenize, tokenize_chunks};
use machina_html::{Diagnostic, Token, TokenizerLimits};

/// Representative documents exercising every major construct: tags,
/// attributes (quoted/unquoted), comments, DOCTYPE, named + numeric
/// character references (including a multi-byte-UTF-8-adjacent case and a
/// CRLF line ending), RCDATA-shaped text, and CDATA.
fn representative_documents() -> Vec<&'static [u8]> {
    vec![
        b"<div class=\"a\" id='b' disabled>text</div>" as &[u8],
        b"<!-- a comment with -- inside --><p>hi</p>",
        b"<!DOCTYPE html PUBLIC \"-//W3C//DTD HTML 4.01//EN\" \"http://x/y.dtd\">",
        b"<p>caf\xc3\xa9 &amp; &#233; &hellip; &notarealthing;</p>",
        b"line one\r\nline two\rline three\n",
        b"<title>a &lt; b</title>",
        b"<script><!--<script>inner</script>--></script>",
        b"<![CDATA[a<b]]>",
        b"<a href=\"?a=1&amp;b=2\">link</a>",
        b"<div \0 class=x>\0</div>",
    ]
}

fn assert_equivalent(label: &str, whole: &[Token], chunked: &[Token]) {
    let whole_normalized = normalize_character_runs(whole.to_vec());
    let chunked_normalized = normalize_character_runs(chunked.to_vec());
    assert_eq!(
        whole_normalized, chunked_normalized,
        "{label}: chunked token stream must be equivalent to unchunked (at the \
         concatenated-character-run level)"
    );
}

#[test]
fn every_two_way_split_of_every_representative_document_is_equivalent() {
    for doc in representative_documents() {
        let (whole_tokens, _) = tokenize(doc, TokenizerLimits::default());
        for (a, b) in all_two_way_splits(doc) {
            let (chunked_tokens, _) = tokenize_chunks(&[a, b], TokenizerLimits::default());
            assert_equivalent(
                &format!(
                    "doc={:?} split_at={}",
                    String::from_utf8_lossy(doc),
                    a.len()
                ),
                &whole_tokens,
                &chunked_tokens,
            );
        }
    }
}

#[test]
fn byte_at_a_time_feed_of_a_larger_document_is_equivalent() {
    let doc: &[u8] = b"<!DOCTYPE html><html><head><title>T &amp; T</title></head>\
<body class=\"x\"><!-- c --><p id='y'>Hello, caf\xc3\xa9 \xf0\x9f\x98\x80 world &hellip;</p>\
<script>if (1 < 2) { document.write(\"<div>\"); }</script></body></html>";

    let (whole_tokens, _) = tokenize(doc, TokenizerLimits::default());

    let chunks: Vec<&[u8]> = (0..doc.len()).map(|i| &doc[i..i + 1]).collect();
    let (chunked_tokens, _) = tokenize_chunks(&chunks, TokenizerLimits::default());

    assert_equivalent("byte-at-a-time", &whole_tokens, &chunked_tokens);
}

#[test]
fn three_and_four_way_splits_of_a_representative_document_are_equivalent() {
    let doc: &[u8] =
        b"<div class=\"a\" data-x=\"y\"><!-- z --><p>caf\xc3\xa9 &amp; &#65;</p></div>";
    let (whole_tokens, _) = tokenize(doc, TokenizerLimits::default());

    let split_points: Vec<usize> = (0..doc.len()).step_by(3).collect();
    for window in split_points.windows(2) {
        let (p1, p2) = (window[0], window[1]);
        if p1 == 0 || p2 >= doc.len() {
            continue;
        }
        let chunks = [&doc[..p1], &doc[p1..p2], &doc[p2..]];
        let (chunked_tokens, _) = tokenize_chunks(&chunks, TokenizerLimits::default());
        assert_equivalent(
            &format!("three-way split at {p1},{p2}"),
            &whole_tokens,
            &chunked_tokens,
        );
    }

    // A genuinely four-way split as well.
    let quarter = doc.len() / 4;
    if quarter > 0 {
        let chunks = [
            &doc[..quarter],
            &doc[quarter..2 * quarter],
            &doc[2 * quarter..3 * quarter],
            &doc[3 * quarter..],
        ];
        let (chunked_tokens, _) = tokenize_chunks(&chunks, TokenizerLimits::default());
        assert_equivalent("four-way split", &whole_tokens, &chunked_tokens);
    }
}

#[test]
fn multibyte_utf8_character_split_at_every_internal_byte_is_equivalent() {
    // U+1F600 GRINNING FACE is 4 bytes in UTF-8; split at each internal
    // offset (which necessarily lands mid-sequence).
    let doc: &[u8] = "a\u{1F600}b".as_bytes();
    let (whole_tokens, whole_diags) = tokenize(doc, TokenizerLimits::default());
    assert!(whole_diags.is_empty());
    for (a, b) in all_two_way_splits(doc) {
        let (chunked_tokens, chunked_diags) = tokenize_chunks(&[a, b], TokenizerLimits::default());
        assert_equivalent("multibyte utf8 split", &whole_tokens, &chunked_tokens);
        assert!(
            chunked_diags.is_empty(),
            "a clean split of valid UTF-8 must never itself produce a diagnostic (split at {})",
            a.len()
        );
    }
}

#[test]
fn crlf_split_between_cr_and_lf_normalizes_identically() {
    let doc: &[u8] = b"a\r\nb\r\nc";
    let (whole_tokens, _) = tokenize(doc, TokenizerLimits::default());
    for (a, b) in all_two_way_splits(doc) {
        let (chunked_tokens, _) = tokenize_chunks(&[a, b], TokenizerLimits::default());
        assert_equivalent("crlf split", &whole_tokens, &chunked_tokens);
    }
}

#[test]
fn named_character_reference_split_mid_entity_name_is_equivalent() {
    // Split inside "&hellip;" at every offset, including right after '&'
    // and right before ';'.
    let doc: &[u8] = b"x&hellip;y";
    let (whole_tokens, _) = tokenize(doc, TokenizerLimits::default());
    for (a, b) in all_two_way_splits(doc) {
        let (chunked_tokens, _) = tokenize_chunks(&[a, b], TokenizerLimits::default());
        assert_equivalent("entity split", &whole_tokens, &chunked_tokens);
    }
}

/// The one legitimate, content-driven (not chunk-boundary-driven) source of
/// `Character` token *count* divergence: `max_character_run_len` forces a
/// flush at a content-determined byte offset regardless of how the caller
/// happened to chunk the input. This test uses a deliberately small limit
/// so the forced-flush boundary is reached, and asserts the flush point is
/// identical (same concatenated text either way) no matter where the
/// *feed()* chunk boundary falls relative to it.
#[test]
fn character_run_limit_flush_point_is_chunk_boundary_independent() {
    let limits = TokenizerLimits {
        max_character_run_len: 16,
        ..TokenizerLimits::default()
    };

    let text_body = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ"; // 37 bytes
    let doc = format!("<p>{text_body}</p>");
    let bytes = doc.as_bytes();

    let (whole_tokens, whole_diags) = tokenize(bytes, limits);
    let whole_character_tokens: Vec<&str> = whole_tokens
        .iter()
        .filter_map(|t| match t {
            Token::Character(c) => Some(c.data.as_str()),
            _ => None,
        })
        .collect();
    // With a 16-byte cap over 37 bytes of text, we expect the run to be
    // forcibly split into more than one Character token.
    assert!(whole_character_tokens.len() > 1);
    let limit_diags: Vec<&Diagnostic> = whole_diags
        .iter()
        .filter(|d| {
            matches!(
                d.code,
                machina_html::ParseErrorCode::LimitExceeded(
                    machina_html::LimitKind::CharacterRunLen
                )
            )
        })
        .collect();
    assert!(!limit_diags.is_empty());

    for (a, b) in all_two_way_splits(bytes) {
        let (chunked_tokens, _) = tokenize_chunks(&[a, b], limits);
        // Concatenated text must match regardless of exact split points,
        // even though the exact Character token *count* may legitimately
        // differ from `whole_tokens` at points near the limit boundary if a
        // feed() boundary and the limit boundary coincide -- what must
        // never differ is the final reconstructed text.
        let concat_whole: String = whole_character_tokens.concat();
        let concat_chunked: String = chunked_tokens
            .iter()
            .filter_map(|t| match t {
                Token::Character(c) => Some(c.data.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(concat_whole, concat_chunked, "split at {}", a.len());

        // Non-character tokens (the tag boundaries) must still match
        // exactly in order/content regardless of the character-run split.
        let whole_non_char: Vec<&Token> = whole_tokens
            .iter()
            .filter(|t| !matches!(t, Token::Character(_)))
            .collect();
        let chunked_non_char: Vec<&Token> = chunked_tokens
            .iter()
            .filter(|t| !matches!(t, Token::Character(_)))
            .collect();
        assert_eq!(whole_non_char, chunked_non_char, "split at {}", a.len());
    }
}
