//! Shared test-only fixture-building helpers. `unwrap`/`expect` here are
//! test-only driving code over fixed, controlled input (mirrors the
//! `crates/dom`/`crates/html-tree-builder` evidence precedent for what
//! "caller-reachable production path" means) — none of this file is
//! reachable from `crates/selectors/src/**`.

#![allow(dead_code)]

use machina_dom::{Document, ElementHandle, NodeHandle};

/// Creates an element, sets its attributes, appends it under `parent`, and
/// returns its handle.
pub fn el(
    document: &mut Document,
    parent: NodeHandle,
    tag: &str,
    attrs: &[(&str, &str)],
) -> ElementHandle {
    let element = document.create_element(tag).expect("valid tag name");
    for (name, value) in attrs {
        document
            .set_attribute(element, name, value)
            .expect("element handle is valid");
    }
    document
        .append_child(parent, element.node_handle())
        .expect("append_child on a fresh detached element cannot fail");
    element
}

/// Appends a text node under `parent`.
pub fn text(document: &mut Document, parent: NodeHandle, data: &str) -> NodeHandle {
    let node = document.create_text(data).node_handle();
    document
        .append_child(parent, node)
        .expect("append_child on a fresh detached text node cannot fail");
    node
}

/// Builds a representative fixture document:
///
/// ```html
/// <html>
///   <body>
///     <div id="main" class="container highlight" data-role="panel">
///       <p class="intro">Hello</p>
///       <ul>
///         <li class="item">One</li>
///         <li class="item selected">Two</li>
///         <li class="item">Three</li>
///       </ul>
///       <span data-testid="row-1" data-role="button">Click</span>
///       <span data-testid="row-2">Other</span>
///     </div>
///     <footer></footer>
///   </body>
/// </html>
/// ```
///
/// Returns `(document, handles)` where `handles` names the elements callers
/// commonly need to assert against directly.
pub struct Fixture {
    pub document: Document,
    pub html: ElementHandle,
    pub body: ElementHandle,
    pub main: ElementHandle,
    pub intro: ElementHandle,
    pub list: ElementHandle,
    pub items: Vec<ElementHandle>,
    pub row1: ElementHandle,
    pub row2: ElementHandle,
    pub footer: ElementHandle,
}

pub fn build_fixture() -> Fixture {
    let mut document = Document::new();
    let root = document.root();
    let html = el(&mut document, root, "html", &[]);
    let body = el(&mut document, html.node_handle(), "body", &[]);
    let main = el(
        &mut document,
        body.node_handle(),
        "div",
        &[
            ("id", "main"),
            ("class", "container highlight"),
            ("data-role", "panel"),
        ],
    );
    let intro = el(
        &mut document,
        main.node_handle(),
        "p",
        &[("class", "intro")],
    );
    text(&mut document, intro.node_handle(), "Hello");

    let list = el(&mut document, main.node_handle(), "ul", &[]);
    let item1 = el(
        &mut document,
        list.node_handle(),
        "li",
        &[("class", "item")],
    );
    text(&mut document, item1.node_handle(), "One");
    let item2 = el(
        &mut document,
        list.node_handle(),
        "li",
        &[("class", "item selected")],
    );
    text(&mut document, item2.node_handle(), "Two");
    let item3 = el(
        &mut document,
        list.node_handle(),
        "li",
        &[("class", "item")],
    );
    text(&mut document, item3.node_handle(), "Three");

    let row1 = el(
        &mut document,
        main.node_handle(),
        "span",
        &[("data-testid", "row-1"), ("data-role", "button")],
    );
    text(&mut document, row1.node_handle(), "Click");
    let row2 = el(
        &mut document,
        main.node_handle(),
        "span",
        &[("data-testid", "row-2")],
    );
    text(&mut document, row2.node_handle(), "Other");

    let footer = el(&mut document, body.node_handle(), "footer", &[]);

    Fixture {
        document,
        html,
        body,
        main,
        intro,
        list,
        items: vec![item1, item2, item3],
        row1,
        row2,
        footer,
    }
}
