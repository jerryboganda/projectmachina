//! Fast-gate item (e): "bounded-output truncation behavior on an oversized
//! document." Exercises both bounded outputs this crate produces: markdown
//! byte-length truncation, and semantic-index item-count truncation. Limit
//! constants (`crates/semantic/src/limits.rs`) are deliberately not
//! re-exported from the crate root (mirrors `crates/selectors`'s own
//! `limits` module being effectively internal-use), so these tests exceed
//! the documented bounds by a comfortable margin rather than hard-coding the
//! exact numeric threshold.

mod support;

use machina_semantic::{extract_semantic_index, generate_markdown};
use support::parse_html;

#[test]
fn markdown_generation_truncates_an_oversized_document_and_flags_it() {
    // MAX_MARKDOWN_BYTES is 200_000 bytes; this paragraph's own text content
    // alone is well over 10x that.
    let big_text = "word ".repeat(100_000); // ~500_000 bytes
    let html = format!("<html><body><p>{big_text}</p></body></html>");
    let doc = parse_html(&html);

    let result =
        generate_markdown(&doc).expect("markdown generation does not error on a large document");
    assert!(result.truncated, "an oversized document must set truncated");
    assert!(
        result.markdown.len() < big_text.len(),
        "truncated markdown must be materially shorter than the untruncated input text"
    );
    assert!(
        result.markdown.contains("truncated"),
        "truncated output must carry a visible truncation marker"
    );
}

#[test]
fn markdown_generation_does_not_truncate_a_small_document() {
    let doc = parse_html("<html><body><p>Short and simple.</p></body></html>");
    let result = generate_markdown(&doc).expect("markdown generation succeeds");
    assert!(!result.truncated);
    assert!(result.markdown.contains("Short and simple."));
}

#[test]
fn semantic_index_truncates_when_heading_count_exceeds_the_item_cap() {
    // MAX_SEMANTIC_ITEMS is 10_000; comfortably exceed it.
    let mut html = String::from("<html><body>");
    for i in 0..10_050 {
        html.push_str(&format!("<h2>H{i}</h2>"));
    }
    html.push_str("</body></html>");
    let doc = parse_html(&html);

    let index = extract_semantic_index(&doc)
        .expect("extraction does not error on a document with many headings");
    assert!(
        index.truncated,
        "exceeding the per-category item cap must set truncated"
    );
    assert!(
        index.headings.len() < 10_050,
        "the headings vector itself must stop growing at the cap, not silently include everything"
    );
}

#[test]
fn semantic_index_does_not_truncate_a_small_document() {
    let doc =
        parse_html("<html><body><h1>One heading</h1><a href=\"/x\">One link</a></body></html>");
    let index = extract_semantic_index(&doc).expect("extraction succeeds");
    assert!(!index.truncated);
}
