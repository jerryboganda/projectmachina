//! Fast-gate item (a): "role/accessible-name derivation for a
//! representative set of elements." Covers native-HTML implicit roles,
//! explicit `role="..."` override, `input[type]` role branching, `<img
//! alt>`, `<label for>`/wrapping-`<label>` accessible-name resolution, and
//! the `<form>` no-text-content-fallback rule.

mod support;

use machina_dom::ElementHandle;
use machina_semantic::{extract_semantic_index, SemanticNode};

use support::parse_html;

fn find(index: &machina_semantic::SemanticIndex, handle: ElementHandle) -> Option<&SemanticNode> {
    index.roles.iter().find(|node| node.handle == handle)
}

const FIXTURE: &str = r#"
<html lang="en"><body>
<h2 id="heading">Section Title</h2>
<a id="link" href="/x">Go there</a>
<a id="plain-anchor">No href</a>
<button id="btn">Click me</button>
<input id="text-input" type="text" placeholder="fallback name">
<input id="checkbox-input" type="checkbox">
<input id="radio-input" type="radio">
<input id="hidden-input" type="hidden" value="x">
<select id="select-single"><option>A</option></select>
<select id="select-multi" multiple><option>A</option></select>
<textarea id="notes"></textarea>
<img id="pic" src="a.png" alt="A cat sitting on a mat">
<img id="pic-no-alt" src="b.png">
<label for="labeled-input">Full Name</label>
<input id="labeled-input" type="text">
<label id="wrap-label">Wrapped Field <input id="wrapped-input" type="text"></label>
<div id="custom-widget" role="button">Custom Button</div>
<form id="form1" action="/submit" method="post">
  <input id="form-input" type="text">
  <button id="form-btn">Submit</button>
</form>
</body></html>
"#;

fn by_id(doc: &machina_dom::Document, id: &str) -> ElementHandle {
    machina_selectors::get_element_by_id(doc, id)
        .expect("query does not error")
        .unwrap_or_else(|| panic!("fixture must contain id={id:?}"))
}

#[test]
fn heading_gets_heading_role_and_level_and_text_name() {
    let doc = parse_html(FIXTURE);
    let index = extract_semantic_index(&doc).expect("extraction succeeds");
    let node = find(&index, by_id(&doc, "heading")).expect("heading indexed");
    assert_eq!(node.role, "heading");
    assert_eq!(node.heading_level, Some(2));
    assert_eq!(node.accessible_name.as_deref(), Some("Section Title"));
}

#[test]
fn anchor_with_href_is_link_without_href_is_not() {
    let doc = parse_html(FIXTURE);
    let index = extract_semantic_index(&doc).expect("extraction succeeds");
    let link = find(&index, by_id(&doc, "link")).expect("a[href] indexed");
    assert_eq!(link.role, "link");
    assert_eq!(link.accessible_name.as_deref(), Some("Go there"));

    let plain = find(&index, by_id(&doc, "plain-anchor"));
    assert!(plain.is_none(), "a without href must not get the link role");
}

#[test]
fn button_role_and_name() {
    let doc = parse_html(FIXTURE);
    let index = extract_semantic_index(&doc).expect("extraction succeeds");
    let node = find(&index, by_id(&doc, "btn")).expect("button indexed");
    assert_eq!(node.role, "button");
    assert_eq!(node.accessible_name.as_deref(), Some("Click me"));
}

#[test]
fn input_type_drives_role_including_hidden_getting_no_role() {
    let doc = parse_html(FIXTURE);
    let index = extract_semantic_index(&doc).expect("extraction succeeds");

    assert_eq!(
        find(&index, by_id(&doc, "checkbox-input")).unwrap().role,
        "checkbox"
    );
    assert_eq!(
        find(&index, by_id(&doc, "radio-input")).unwrap().role,
        "radio"
    );
    assert!(
        find(&index, by_id(&doc, "hidden-input")).is_none(),
        "input[type=hidden] must not get a role"
    );
}

#[test]
fn select_role_depends_on_multiple_attribute() {
    let doc = parse_html(FIXTURE);
    let index = extract_semantic_index(&doc).expect("extraction succeeds");
    assert_eq!(
        find(&index, by_id(&doc, "select-single")).unwrap().role,
        "combobox"
    );
    assert_eq!(
        find(&index, by_id(&doc, "select-multi")).unwrap().role,
        "listbox"
    );
}

#[test]
fn textarea_role_is_textbox() {
    let doc = parse_html(FIXTURE);
    let index = extract_semantic_index(&doc).expect("extraction succeeds");
    assert_eq!(find(&index, by_id(&doc, "notes")).unwrap().role, "textbox");
}

#[test]
fn img_role_and_alt_derived_name() {
    let doc = parse_html(FIXTURE);
    let index = extract_semantic_index(&doc).expect("extraction succeeds");
    let with_alt = find(&index, by_id(&doc, "pic")).unwrap();
    assert_eq!(with_alt.role, "img");
    assert_eq!(
        with_alt.accessible_name.as_deref(),
        Some("A cat sitting on a mat")
    );
    let without_alt = find(&index, by_id(&doc, "pic-no-alt")).unwrap();
    assert_eq!(without_alt.accessible_name, None);
}

#[test]
fn label_for_resolves_accessible_name() {
    let doc = parse_html(FIXTURE);
    let index = extract_semantic_index(&doc).expect("extraction succeeds");
    let node = find(&index, by_id(&doc, "labeled-input")).unwrap();
    assert_eq!(node.accessible_name.as_deref(), Some("Full Name"));
}

#[test]
fn wrapping_label_resolves_accessible_name_when_no_for_attribute_matches() {
    let doc = parse_html(FIXTURE);
    let index = extract_semantic_index(&doc).expect("extraction succeeds");
    let node = find(&index, by_id(&doc, "wrapped-input")).unwrap();
    assert_eq!(node.accessible_name.as_deref(), Some("Wrapped Field"));
}

#[test]
fn placeholder_is_last_resort_name_for_form_controls() {
    let doc = parse_html(FIXTURE);
    let index = extract_semantic_index(&doc).expect("extraction succeeds");
    let node = find(&index, by_id(&doc, "text-input")).unwrap();
    assert_eq!(node.accessible_name.as_deref(), Some("fallback name"));
}

#[test]
fn explicit_role_attribute_overrides_implicit_mapping() {
    let doc = parse_html(FIXTURE);
    let index = extract_semantic_index(&doc).expect("extraction succeeds");
    let node = find(&index, by_id(&doc, "custom-widget")).unwrap();
    assert_eq!(node.role, "button");
    assert_eq!(node.accessible_name.as_deref(), Some("Custom Button"));
}

#[test]
fn form_role_and_no_generic_text_fallback_name() {
    let doc = parse_html(FIXTURE);
    let index = extract_semantic_index(&doc).expect("extraction succeeds");
    let node = find(&index, by_id(&doc, "form1")).unwrap();
    assert_eq!(node.role, "form");
    assert_eq!(
        node.accessible_name, None,
        "a form's descendant text must not become its accessible name"
    );
}
