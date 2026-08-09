//! Fast-gate smoke substitute for the deferred `cargo-fuzz` corpus (see
//! `.agent-state/evidence/M2-T03.md`): a small, dependency-free randomized
//! test that feeds thousands of pseudo-random byte sequences — biased
//! toward HTML-meaningful bytes to actually exercise interesting states —
//! through the tokenizer and asserts it never panics and always terminates
//! (every run calls `finish()` and drains to `Token::Eof`). This is not a
//! substitute for real `cargo-fuzz` coverage-guided fuzzing (deferred), but
//! it is real, reproducible (fixed seed) coverage of "crash-free on
//! adversarial-ish input" beyond the hand-picked cases in
//! `bounded_limits.rs`.

mod common;

use common::tokenize;
use machina_html::{Token, TokenizerLimits};

/// A tiny deterministic xorshift PRNG -- no external dependency needed for
/// a fixed-seed smoke test.
struct Xorshift(u64);

impl Xorshift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn next_usize(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }
}

const ALPHABET: &[u8] =
    b"<>/!-\"'=& #x0123456789abcdefghijklmnopqrstuvwxyzDOCTYPECDATAscript\0\r\n\t;";

fn random_document(rng: &mut Xorshift, len: usize) -> Vec<u8> {
    (0..len)
        .map(|_| ALPHABET[rng.next_usize(ALPHABET.len())])
        .collect()
}

fn assert_terminates_cleanly(input: &[u8]) {
    let (tokens, _diags) = tokenize(input, TokenizerLimits::default());
    assert!(
        matches!(tokens.last(), Some(Token::Eof)),
        "tokenizer did not terminate cleanly for input {input:?}"
    );
}

#[test]
fn random_html_like_byte_soup_never_panics_and_always_terminates() {
    let mut rng = Xorshift(0x9E3779B97F4A7C15);
    for _ in 0..3000 {
        let len = rng.next_usize(64);
        let doc = random_document(&mut rng, len);
        assert_terminates_cleanly(&doc);
    }
}

#[test]
fn random_html_like_byte_soup_chunked_arbitrarily_never_panics() {
    let mut rng = Xorshift(0xD1B54A32D192ED03);
    for _ in 0..500 {
        let len = rng.next_usize(200);
        let doc = random_document(&mut rng, len);
        let mut tokenizer = machina_html::Tokenizer::new(TokenizerLimits::default());
        let mut offset = 0;
        while offset < doc.len() {
            let step = 1 + rng.next_usize(7);
            let end = (offset + step).min(doc.len());
            tokenizer.feed(&doc[offset..end]);
            let mut iterations = 0;
            while let Some(_event) = tokenizer.next_event() {
                iterations += 1;
                assert!(
                    iterations < 1_000_000,
                    "possible infinite event loop for input {doc:?}"
                );
            }
            offset = end;
        }
        tokenizer.finish();
        let mut saw_eof = false;
        let mut iterations = 0;
        while let Some(event) = tokenizer.next_event() {
            iterations += 1;
            assert!(
                iterations < 1_000_000,
                "possible infinite event loop for input {doc:?}"
            );
            if matches!(event, machina_html::TokenizerEvent::Token(Token::Eof)) {
                saw_eof = true;
            }
        }
        assert!(saw_eof, "tokenizer never reached Eof for input {doc:?}");
    }
}

#[test]
fn arbitrary_bytes_including_invalid_utf8_never_panic() {
    let mut rng = Xorshift(0x2545F4914F6CDD1D);
    for _ in 0..2000 {
        let len = rng.next_usize(48);
        let doc: Vec<u8> = (0..len).map(|_| (rng.next() % 256) as u8).collect();
        assert_terminates_cleanly(&doc);
    }
}
