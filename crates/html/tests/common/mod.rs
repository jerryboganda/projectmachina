//! Shared test helpers for the `machina-html` integration test suite.
//!
//! Each `tests/*.rs` file is compiled as its own separate crate, and not
//! every helper here is used by every consumer -- `allow(dead_code)` below
//! is scoped to this shared module for exactly that reason, not to hide a
//! real unused-code issue in the crate under test.
#![allow(dead_code)]

use machina_html::{Diagnostic, Token, TokenizerEvent, TokenizerLimits};

/// Tokenize `bytes` in one `feed()` call, returning tokens and diagnostics
/// separately, in original interleaved order preserved within each list.
pub fn tokenize(bytes: &[u8], limits: TokenizerLimits) -> (Vec<Token>, Vec<Diagnostic>) {
    tokenize_chunks(&[bytes], limits)
}

/// Tokenize `chunks` fed across multiple `feed()` calls (in order), then
/// `finish()`.
pub fn tokenize_chunks(chunks: &[&[u8]], limits: TokenizerLimits) -> (Vec<Token>, Vec<Diagnostic>) {
    let mut tokenizer = machina_html::Tokenizer::new(limits);
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();
    for chunk in chunks {
        tokenizer.feed(chunk);
        drain(&mut tokenizer, &mut tokens, &mut diagnostics);
    }
    tokenizer.finish();
    drain(&mut tokenizer, &mut tokens, &mut diagnostics);
    (tokens, diagnostics)
}

fn drain(
    tokenizer: &mut machina_html::Tokenizer,
    tokens: &mut Vec<Token>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    while let Some(event) = tokenizer.next_event() {
        match event {
            TokenizerEvent::Token(token) => tokens.push(token),
            TokenizerEvent::Diagnostic(diagnostic) => diagnostics.push(diagnostic),
        }
    }
}

/// Merge consecutive `Character` tokens into one, per the streaming
/// equivalence contract (design §3): chunked vs. unchunked input must
/// produce an equivalent token stream at the *concatenated-text* level
/// between non-`Character` tokens, not necessarily an identical token
/// *count*.
pub fn normalize_character_runs(tokens: Vec<Token>) -> Vec<Token> {
    let mut normalized: Vec<Token> = Vec::new();
    for token in tokens {
        if let Token::Character(c) = &token {
            if let Some(Token::Character(prev)) = normalized.last_mut() {
                prev.data.push_str(&c.data);
                continue;
            }
        }
        normalized.push(token);
    }
    normalized
}

/// All byte offsets `0..=bytes.len()` split points, used to build every
/// two-way chunking of `bytes` for exhaustive chunk-boundary tests.
pub fn all_two_way_splits(bytes: &[u8]) -> Vec<(&[u8], &[u8])> {
    (0..=bytes.len()).map(|i| bytes.split_at(i)).collect()
}
