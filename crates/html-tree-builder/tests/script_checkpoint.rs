//! Acceptance criterion: "parser can pause/resume around script
//! checkpoints" (design §6). Covers: a checkpoint fires exactly at each
//! script's end tag with the element + its text child already built;
//! `resume_after_script` continues parsing correctly; out-of-turn calls
//! (`feed`/`finish` while paused, `resume_after_script` while not paused)
//! return the documented typed errors rather than panicking or silently
//! no-opping; different chunk splits produce an equivalent checkpoint
//! (same script content, same source classification).

mod support;

use machina_dom::Document;
use machina_html::{Tokenizer, TokenizerLimits};
use machina_html_tree_builder::{ScriptSource, TreeBuilder, TreeBuilderError, TreeBuilderOutcome};

fn drain_to_next_outcome(
    doc: &mut Document,
    tokenizer: &mut Tokenizer,
    builder: &mut TreeBuilder,
    mut outcome: TreeBuilderOutcome,
) -> TreeBuilderOutcome {
    while let TreeBuilderOutcome::NeedsMoreInput = outcome {
        outcome = builder
            .finish(doc, tokenizer)
            .expect("finish should not error");
    }
    outcome
}

#[test]
fn inline_script_end_tag_produces_a_checkpoint_with_the_dom_already_built() {
    let mut doc = Document::new();
    let mut tokenizer = Tokenizer::new(TokenizerLimits::default());
    let mut builder = TreeBuilder::new(false);

    let outcome = builder
        .feed(
            &mut doc,
            &mut tokenizer,
            b"<html><body><script>var x=1;</script>after</body></html>",
        )
        .expect("feed should not error");
    let outcome = drain_to_next_outcome(&mut doc, &mut tokenizer, &mut builder, outcome);

    let TreeBuilderOutcome::ScriptCheckpoint(checkpoint) = outcome else {
        panic!("expected ScriptCheckpoint, got {outcome:?}");
    };
    assert_eq!(checkpoint.source, ScriptSource::Inline);
    assert!(builder.is_paused());

    let children = doc
        .children(checkpoint.script_element.node_handle())
        .expect("script element should resolve");
    assert_eq!(
        children.len(),
        1,
        "script's text child should already be built"
    );
    assert_eq!(doc.text_data(children[0]).unwrap(), "var x=1;");
}

#[test]
fn external_script_is_classified_via_the_src_attribute() {
    let mut doc = Document::new();
    let mut tokenizer = Tokenizer::new(TokenizerLimits::default());
    let mut builder = TreeBuilder::new(false);

    let outcome = builder
        .feed(&mut doc, &mut tokenizer, b"<script src=\"a.js\"></script>")
        .expect("feed should not error");
    let outcome = drain_to_next_outcome(&mut doc, &mut tokenizer, &mut builder, outcome);

    let TreeBuilderOutcome::ScriptCheckpoint(checkpoint) = outcome else {
        panic!("expected ScriptCheckpoint, got {outcome:?}");
    };
    assert_eq!(checkpoint.source, ScriptSource::External);
}

#[test]
fn feed_and_finish_return_a_typed_error_while_paused() {
    let mut doc = Document::new();
    let mut tokenizer = Tokenizer::new(TokenizerLimits::default());
    let mut builder = TreeBuilder::new(false);

    let outcome = builder
        .feed(&mut doc, &mut tokenizer, b"<script>1;</script>")
        .expect("feed should not error");
    let outcome = drain_to_next_outcome(&mut doc, &mut tokenizer, &mut builder, outcome);
    assert!(matches!(outcome, TreeBuilderOutcome::ScriptCheckpoint(_)));

    assert_eq!(
        builder.feed(&mut doc, &mut tokenizer, b"more"),
        Err(TreeBuilderError::AlreadyPaused)
    );
    assert_eq!(
        builder.finish(&mut doc, &mut tokenizer),
        Err(TreeBuilderError::AlreadyPaused)
    );
}

#[test]
fn resume_after_script_while_not_paused_returns_a_typed_error() {
    let mut doc = Document::new();
    let mut tokenizer = Tokenizer::new(TokenizerLimits::default());
    let mut builder = TreeBuilder::new(false);

    // Never paused at all: an empty document reaches `Done` immediately.
    let outcome = builder
        .feed(&mut doc, &mut tokenizer, b"<html></html>")
        .expect("feed should not error");
    let _ = drain_to_next_outcome(&mut doc, &mut tokenizer, &mut builder, outcome);

    assert_eq!(
        builder.resume_after_script(&mut doc, &mut tokenizer),
        Err(TreeBuilderError::NotPaused)
    );
}

#[test]
fn resume_after_script_continues_parsing_and_a_second_call_is_rejected() {
    let mut doc = Document::new();
    let mut tokenizer = Tokenizer::new(TokenizerLimits::default());
    let mut builder = TreeBuilder::new(false);

    let outcome = builder
        .feed(
            &mut doc,
            &mut tokenizer,
            b"<html><body><script>1;</script><p>done</p></body></html>",
        )
        .expect("feed should not error");
    let outcome = drain_to_next_outcome(&mut doc, &mut tokenizer, &mut builder, outcome);
    assert!(matches!(outcome, TreeBuilderOutcome::ScriptCheckpoint(_)));

    let outcome = builder
        .resume_after_script(&mut doc, &mut tokenizer)
        .expect("resume_after_script should succeed exactly once while paused");
    let outcome = drain_to_next_outcome(&mut doc, &mut tokenizer, &mut builder, outcome);
    assert_eq!(outcome, TreeBuilderOutcome::Done);

    assert_eq!(
        builder.resume_after_script(&mut doc, &mut tokenizer),
        Err(TreeBuilderError::NotPaused)
    );

    let html = builder.document_element().expect("html element inserted");
    let rendered = support::render(&doc, html.node_handle());
    assert!(
        rendered.contains("<p>done</p>"),
        "content after the resumed script should still parse: {rendered}"
    );
}

#[test]
fn checkpoint_script_text_is_equivalent_across_different_chunk_splits() {
    let html: &[u8] = b"<html><body><script>1+1;</script></body></html>";

    let mut doc_whole = Document::new();
    let mut tok_whole = Tokenizer::new(TokenizerLimits::default());
    let mut builder_whole = TreeBuilder::new(false);
    let outcome = builder_whole
        .feed(&mut doc_whole, &mut tok_whole, html)
        .expect("feed should not error");
    let outcome =
        drain_to_next_outcome(&mut doc_whole, &mut tok_whole, &mut builder_whole, outcome);
    let TreeBuilderOutcome::ScriptCheckpoint(cp_whole) = outcome else {
        panic!("expected ScriptCheckpoint (whole), got {outcome:?}");
    };
    let text_whole = doc_whole
        .text_data(
            doc_whole
                .children(cp_whole.script_element.node_handle())
                .unwrap()[0],
        )
        .unwrap()
        .to_string();

    let mut doc_chunked = Document::new();
    let mut tok_chunked = Tokenizer::new(TokenizerLimits::default());
    let mut builder_chunked = TreeBuilder::new(false);
    let mut outcome = TreeBuilderOutcome::NeedsMoreInput;
    for byte in html {
        outcome = builder_chunked
            .feed(
                &mut doc_chunked,
                &mut tok_chunked,
                std::slice::from_ref(byte),
            )
            .expect("feed should not error");
        if matches!(outcome, TreeBuilderOutcome::ScriptCheckpoint(_)) {
            break;
        }
    }
    let outcome = drain_to_next_outcome(
        &mut doc_chunked,
        &mut tok_chunked,
        &mut builder_chunked,
        outcome,
    );
    let TreeBuilderOutcome::ScriptCheckpoint(cp_chunked) = outcome else {
        panic!("expected ScriptCheckpoint (chunked), got {outcome:?}");
    };
    let text_chunked = doc_chunked
        .text_data(
            doc_chunked
                .children(cp_chunked.script_element.node_handle())
                .unwrap()[0],
        )
        .unwrap()
        .to_string();

    assert_eq!(text_whole, text_chunked);
    assert_eq!(cp_whole.source, cp_chunked.source);
}
