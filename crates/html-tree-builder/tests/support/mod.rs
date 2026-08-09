//! Shared test helpers for the `machina-html-tree-builder` integration test
//! suite. Each `tests/*.rs` file is compiled as its own separate crate, and
//! not every helper here is used by every consumer — `allow(dead_code)`
//! below is scoped to this shared module for exactly that reason, matching
//! `crates/html/tests/common/mod.rs`'s precedent.
#![allow(dead_code)]

use machina_dom::{Document, NodeHandle, NodeKind};
use machina_html::{Tokenizer, TokenizerLimits};
use machina_html_tree_builder::{TreeBuilder, TreeBuilderOutcome};

/// Drives `builder` to `Done`, transparently calling `finish` when the
/// tokenizer runs out of buffered input and `resume_after_script` at every
/// script checkpoint (tests that care about checkpoints drive the builder
/// manually instead of using this helper).
pub fn parse_to_completion(html: &str) -> (Document, TreeBuilder) {
    let mut doc = Document::new();
    let mut tokenizer = Tokenizer::new(TokenizerLimits::default());
    let mut builder = TreeBuilder::new(false);
    let mut outcome = builder
        .feed(&mut doc, &mut tokenizer, html.as_bytes())
        .expect("feed should not error on well-formed driving");
    loop {
        outcome = match outcome {
            TreeBuilderOutcome::NeedsMoreInput => builder
                .finish(&mut doc, &mut tokenizer)
                .expect("finish should not error"),
            TreeBuilderOutcome::ScriptCheckpoint(_) => builder
                .resume_after_script(&mut doc, &mut tokenizer)
                .expect("resume_after_script should not error while paused"),
            TreeBuilderOutcome::Done => break,
        };
    }
    (doc, builder)
}

/// Same as [`parse_to_completion`] but feeds one byte per `feed()` call —
/// used by chunk-boundary-equivalence tests.
pub fn parse_to_completion_byte_at_a_time(html: &str) -> (Document, TreeBuilder) {
    let mut doc = Document::new();
    let mut tokenizer = Tokenizer::new(TokenizerLimits::default());
    let mut builder = TreeBuilder::new(false);
    for byte in html.as_bytes() {
        let mut outcome = builder
            .feed(&mut doc, &mut tokenizer, std::slice::from_ref(byte))
            .expect("feed should not error on well-formed driving");
        while let TreeBuilderOutcome::ScriptCheckpoint(_) = outcome {
            outcome = builder
                .resume_after_script(&mut doc, &mut tokenizer)
                .expect("resume_after_script should not error while paused");
        }
    }
    let mut outcome = TreeBuilderOutcome::NeedsMoreInput;
    loop {
        outcome = match outcome {
            TreeBuilderOutcome::NeedsMoreInput => builder
                .finish(&mut doc, &mut tokenizer)
                .expect("finish should not error"),
            TreeBuilderOutcome::ScriptCheckpoint(_) => builder
                .resume_after_script(&mut doc, &mut tokenizer)
                .expect("resume_after_script should not error while paused"),
            TreeBuilderOutcome::Done => break,
        };
    }
    (doc, builder)
}

/// Renders `node` (and its descendants) into a compact, comparison-friendly
/// outline: `<tag>children</tag>` for elements, raw text for text nodes,
/// children-only (no wrapper) for comments/doctype/document/fragment nodes.
/// Attributes are intentionally omitted — tests that care about attribute
/// values read them directly via `Document::attribute`.
/// Iterative (explicit stack) on purpose: fixtures deliberately include
/// adversarially deep nesting, and a naive recursive walk of *this test
/// helper* (not the tree builder under test, which is itself iterative —
/// design §7f) would overflow the test binary's own stack before it ever
/// got to assert anything.
pub fn render(doc: &Document, node: NodeHandle) -> String {
    enum Frame {
        Open(NodeHandle),
        CloseElement(String),
    }

    let mut out = String::new();
    let mut stack = vec![Frame::Open(node)];
    while let Some(frame) = stack.pop() {
        match frame {
            Frame::CloseElement(tag) => {
                out.push_str("</");
                out.push_str(&tag);
                out.push('>');
            }
            Frame::Open(handle) => {
                let Ok(node_ref) = doc.node(handle) else {
                    continue;
                };
                match node_ref.kind() {
                    NodeKind::Element => {
                        let element = doc.as_element(handle).expect("kind() said Element");
                        let tag = doc.tag_name(element).unwrap_or("?").to_string();
                        out.push('<');
                        out.push_str(&tag);
                        out.push('>');
                        stack.push(Frame::CloseElement(tag));
                        if let Ok(children) = doc.children(handle) {
                            for child in children.into_iter().rev() {
                                stack.push(Frame::Open(child));
                            }
                        }
                    }
                    NodeKind::Text => {
                        if let Ok(text) = doc.text_data(handle) {
                            out.push_str(text);
                        }
                    }
                    NodeKind::Comment
                    | NodeKind::DocumentType
                    | NodeKind::Document
                    | NodeKind::DocumentFragment => {
                        if let Ok(children) = doc.children(handle) {
                            for child in children.into_iter().rev() {
                                stack.push(Frame::Open(child));
                            }
                        }
                    }
                }
            }
        }
    }
    out
}
