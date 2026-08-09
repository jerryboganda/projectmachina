//! Fast-gate item (c): "markdown output for a representative document
//! (headings, paragraph, list, link, bold/italic, code)."

mod support;

use machina_semantic::generate_markdown;
use support::parse_html;

#[test]
fn representative_document_produces_expected_readable_markdown() {
    let html = r#"
<html><head><title>Ignored</title></head><body>
<h1>Main Title</h1>
<p>This is <strong>bold</strong> and <em>italic</em> text with a <a href="/x">link</a>.</p>
<ul>
<li>One</li>
<li>Two</li>
</ul>
<p>Inline <code>code()</code> example.</p>
</body></html>
"#;
    let doc = parse_html(html);
    let result = generate_markdown(&doc).expect("markdown generation succeeds");
    assert!(!result.truncated);

    assert!(
        result.markdown.contains("# Main Title"),
        "markdown was:\n{}",
        result.markdown
    );
    assert!(result.markdown.contains("**bold**"));
    assert!(result.markdown.contains("*italic*"));
    assert!(result.markdown.contains("[link](/x)"));
    assert!(result.markdown.contains("- One"));
    assert!(result.markdown.contains("- Two"));
    assert!(result.markdown.contains("`code()`"));
    // <title> text (head content) must never leak into the body markdown.
    assert!(!result.markdown.contains("Ignored"));
}

#[test]
fn ordered_and_nested_lists_render_with_indentation() {
    let html = r#"
<html><body>
<ol>
<li>First</li>
<li>Second
  <ul><li>Nested</li></ul>
</li>
<li>Third</li>
</ol>
</body></html>
"#;
    let doc = parse_html(html);
    let result = generate_markdown(&doc).expect("markdown generation succeeds");
    assert!(result.markdown.contains("1. First"));
    assert!(result.markdown.contains("2. Second"));
    assert!(result.markdown.contains("3. Third"));
    assert!(
        result.markdown.contains("- Nested"),
        "markdown was:\n{}",
        result.markdown
    );
}

#[test]
fn pre_block_preserves_whitespace_inside_a_fence() {
    let html = "<html><body><pre>line one\n  indented line two</pre></body></html>";
    let doc = parse_html(html);
    let result = generate_markdown(&doc).expect("markdown generation succeeds");
    assert!(result.markdown.contains("```"));
    assert!(result.markdown.contains("line one\n  indented line two"));
}

#[test]
fn heading_levels_one_through_six_map_to_hash_counts() {
    let html =
        "<html><body><h1>A</h1><h2>B</h2><h3>C</h3><h4>D</h4><h5>E</h5><h6>F</h6></body></html>";
    let doc = parse_html(html);
    let result = generate_markdown(&doc).expect("markdown generation succeeds");
    for (hashes, text) in [
        ("#", "A"),
        ("##", "B"),
        ("###", "C"),
        ("####", "D"),
        ("#####", "E"),
        ("######", "F"),
    ] {
        let expected = format!("{hashes} {text}");
        assert!(
            result.markdown.contains(&expected),
            "expected {expected:?} in:\n{}",
            result.markdown
        );
    }
}

#[test]
fn script_and_style_content_never_appears_in_markdown() {
    let html = r#"<html><head><style>body{color:red}</style></head>
<body><script>alert('should not appear')</script><p>Visible text</p></body></html>"#;
    let doc = parse_html(html);
    let result = generate_markdown(&doc).expect("markdown generation succeeds");
    assert!(result.markdown.contains("Visible text"));
    assert!(!result.markdown.contains("color:red"));
    assert!(!result.markdown.contains("should not appear"));
}

#[test]
fn image_renders_as_markdown_image_syntax() {
    let html = r#"<html><body><img src="cat.png" alt="A cat"></body></html>"#;
    let doc = parse_html(html);
    let result = generate_markdown(&doc).expect("markdown generation succeeds");
    assert_eq!(result.markdown, "![A cat](cat.png)");
}
