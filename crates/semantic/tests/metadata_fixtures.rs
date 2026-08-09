//! Metadata/schema extraction fixtures: title, `<html lang>`, and every
//! `<meta>` tag shape this crate supports (`charset`, `name`+`content`,
//! `property`+`content` for Open-Graph-style tags, `http-equiv`+`content`).

mod support;

use machina_semantic::extract_metadata;
use support::parse_html;

const FIXTURE: &str = r#"
<html lang="en-US"><head>
<meta charset="utf-8">
<title>  Example   Page  </title>
<meta name="description" content="A page about examples.">
<meta property="og:title" content="Example Page (OG)">
<meta http-equiv="refresh" content="30">
<meta name="empty-content-ignored">
</head><body></body></html>
"#;

#[test]
fn title_is_extracted_and_whitespace_normalized() {
    let doc = parse_html(FIXTURE);
    let metadata = extract_metadata(&doc).expect("metadata extraction succeeds");
    assert_eq!(metadata.title.as_deref(), Some("Example Page"));
}

#[test]
fn html_lang_attribute_is_extracted() {
    let doc = parse_html(FIXTURE);
    let metadata = extract_metadata(&doc).expect("metadata extraction succeeds");
    assert_eq!(metadata.lang.as_deref(), Some("en-US"));
}

#[test]
fn every_meta_tag_shape_is_captured_as_a_key_value_pair() {
    let doc = parse_html(FIXTURE);
    let metadata = extract_metadata(&doc).expect("metadata extraction succeeds");
    let map: std::collections::HashMap<_, _> = metadata.meta.into_iter().collect();

    assert_eq!(map.get("charset").map(String::as_str), Some("utf-8"));
    assert_eq!(
        map.get("description").map(String::as_str),
        Some("A page about examples.")
    );
    assert_eq!(
        map.get("og:title").map(String::as_str),
        Some("Example Page (OG)")
    );
    assert_eq!(map.get("refresh").map(String::as_str), Some("30"));
}

#[test]
fn meta_tag_with_no_content_attribute_is_skipped_not_errored() {
    let doc = parse_html(FIXTURE);
    let metadata = extract_metadata(&doc).expect("metadata extraction succeeds");
    assert!(!metadata
        .meta
        .iter()
        .any(|(key, _)| key == "empty-content-ignored"));
}

#[test]
fn document_with_no_head_content_produces_all_none_gracefully() {
    let doc = parse_html("<html><body><p>No head metadata here.</p></body></html>");
    let metadata = extract_metadata(&doc).expect("metadata extraction succeeds");
    assert_eq!(metadata.title, None);
    assert_eq!(metadata.lang, None);
    assert!(metadata.meta.is_empty());
    assert!(!metadata.truncated);
}
