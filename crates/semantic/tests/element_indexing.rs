//! Fast-gate item (b): "heading/link/form/interactive-element extraction
//! ordering and correctness."

mod support;

use machina_dom::ElementHandle;
use machina_semantic::extract_semantic_index;

use support::parse_html;

const FIXTURE: &str = r#"
<html><body>
<h1 id="h1">Title</h1>
<p>Intro <a id="a1" href="/one">One</a> and <a id="a2" href="/two">Two</a>.</p>
<h2 id="h2">Section</h2>
<form id="form-a" action="/submit-a">
  <input id="form-a-input" type="text">
  <button id="form-a-btn">Go</button>
</form>
<div>
  <form id="form-b">
    <select id="form-b-select"><option>x</option></select>
  </form>
</div>
<a id="a3" href="/three">Three</a>
<button id="loose-btn">Loose</button>
<input id="loose-checkbox" type="checkbox">
</body></html>
"#;

fn by_id(doc: &machina_dom::Document, id: &str) -> ElementHandle {
    machina_selectors::get_element_by_id(doc, id)
        .expect("query does not error")
        .unwrap_or_else(|| panic!("fixture must contain id={id:?}"))
}

#[test]
fn headings_are_collected_in_document_order() {
    let doc = parse_html(FIXTURE);
    let index = extract_semantic_index(&doc).expect("extraction succeeds");
    let handles: Vec<_> = index.headings.iter().map(|node| node.handle).collect();
    assert_eq!(handles, vec![by_id(&doc, "h1"), by_id(&doc, "h2")]);
    assert_eq!(index.headings[0].heading_level, Some(1));
    assert_eq!(index.headings[1].heading_level, Some(2));
}

#[test]
fn links_are_collected_in_document_order_with_raw_href() {
    let doc = parse_html(FIXTURE);
    let index = extract_semantic_index(&doc).expect("extraction succeeds");
    let hrefs: Vec<_> = index.links.iter().map(|link| link.href.as_str()).collect();
    assert_eq!(hrefs, vec!["/one", "/two", "/three"]);
    assert_eq!(index.links[0].accessible_name.as_deref(), Some("One"));
}

#[test]
fn forms_are_collected_with_descendant_controls_only() {
    let doc = parse_html(FIXTURE);
    let index = extract_semantic_index(&doc).expect("extraction succeeds");
    assert_eq!(index.forms.len(), 2);

    let form_a = index
        .forms
        .iter()
        .find(|form| form.handle == by_id(&doc, "form-a"))
        .expect("form-a indexed");
    assert_eq!(form_a.action.as_deref(), Some("/submit-a"));
    assert_eq!(
        form_a.controls,
        vec![by_id(&doc, "form-a-input"), by_id(&doc, "form-a-btn")]
    );

    let form_b = index
        .forms
        .iter()
        .find(|form| form.handle == by_id(&doc, "form-b"))
        .expect("form-b indexed (nested inside a div)");
    assert_eq!(form_b.controls, vec![by_id(&doc, "form-b-select")]);
}

#[test]
fn interactive_elements_include_every_interactive_role_document_wide() {
    let doc = parse_html(FIXTURE);
    let index = extract_semantic_index(&doc).expect("extraction succeeds");
    let handles: std::collections::HashSet<_> = index
        .interactive_elements
        .iter()
        .map(|node| node.handle)
        .collect();

    for id in [
        "a1",
        "a2",
        "a3",
        "form-a-input",
        "form-a-btn",
        "form-b-select",
        "loose-btn",
        "loose-checkbox",
    ] {
        assert!(
            handles.contains(&by_id(&doc, id)),
            "expected id={id:?} to be flagged interactive"
        );
    }
    // Headings/forms themselves are not interactive roles.
    assert!(!handles.contains(&by_id(&doc, "h1")));
    assert!(!handles.contains(&by_id(&doc, "form-a")));
}

#[test]
fn extraction_reports_no_truncation_for_a_small_document() {
    let doc = parse_html(FIXTURE);
    let index = extract_semantic_index(&doc).expect("extraction succeeds");
    assert!(!index.truncated);
}
