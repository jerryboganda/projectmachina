//! Shared test-only fixture-building helpers. `unwrap`/`expect` here are
//! test-only driving code over fixed, controlled input (mirrors the
//! `crates/dom`/`crates/html-tree-builder`/`crates/selectors` evidence
//! precedent for what "caller-reachable production path" means) — none of
//! this file is reachable from `crates/semantic/src/**`.
#![allow(dead_code)]

use machina_dom::Document;
use machina_html::{Tokenizer, TokenizerLimits};
use machina_html_tree_builder::{TreeBuilder, TreeBuilderOutcome};

/// Parses `html` to completion via the real tokenizer and tree builder
/// (M2-T03, M2-T04), transparently driving `finish`/`resume_after_script`.
/// Building fixtures this way, rather than constructing elements one at a
/// time via `Document::create_element`, exercises this crate against
/// realistic, browser-parsed document shapes (implicit
/// `<html>`/`<head>`/`<body>`, auto-closed tags, ...).
pub fn parse_html(html: &str) -> Document {
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
    doc
}
