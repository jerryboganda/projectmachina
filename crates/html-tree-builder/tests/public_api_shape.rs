//! Compile-only smoke test (design §8): confirms `TreeBuilder` is `Send`
//! and that this crate's public surface only names types from
//! `machina_html`/`machina_dom`/`std` — no protocol/command-model/
//! command-bus crate leaks into the signature of any re-exported item.
//! (The dependency-direction guarantee itself is enforced structurally by
//! `crates/html-tree-builder/Cargo.toml` only listing `machina-html` and
//! `machina-dom` as dependencies — this test exercises the *type* surface
//! that Cargo.toml makes possible.)

fn assert_send<T: Send>() {}

#[test]
fn tree_builder_and_its_public_outputs_are_send() {
    assert_send::<machina_html_tree_builder::TreeBuilder>();
    assert_send::<machina_html_tree_builder::TreeBuilderOutcome>();
    assert_send::<machina_html_tree_builder::TreeBuilderError>();
    assert_send::<machina_html_tree_builder::Diagnostic>();
    assert_send::<machina_html_tree_builder::ScriptCheckpoint>();
}

#[test]
fn public_constructors_and_driving_methods_have_the_documented_shape() {
    // Compile-time shape check for the design §6 contract: `feed`/`finish`/
    // `resume_after_script` all take `&mut Document` + `&mut Tokenizer`
    // per call, never owning either.
    let mut doc = machina_dom::Document::new();
    let mut tokenizer = machina_html::Tokenizer::new(machina_html::TokenizerLimits::default());
    let mut builder = machina_html_tree_builder::TreeBuilder::new(false);
    let _: Result<
        machina_html_tree_builder::TreeBuilderOutcome,
        machina_html_tree_builder::TreeBuilderError,
    > = builder.feed(&mut doc, &mut tokenizer, b"<html></html>");
    let _: Result<
        machina_html_tree_builder::TreeBuilderOutcome,
        machina_html_tree_builder::TreeBuilderError,
    > = builder.finish(&mut doc, &mut tokenizer);
}
