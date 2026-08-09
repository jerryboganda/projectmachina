//! `machina-html` — a streaming, bounded-input HTML tokenizer for Project
//! Machina's native engine (M2-T03).
//!
//! Implements the WHATWG HTML §13.2.5 tokenization state machine: text-
//! content modes, tag/attribute parsing, comments, DOCTYPE, a curated
//! subset of named character references plus full numeric character
//! references, and a foreign-content CDATA section. See
//! `.agent-state/design/M2-T03-html-tokenizer-design.md` for the full
//! design this crate implements, and `.agent-state/evidence/M2-T03.md` for
//! exactly what shipped versus what remains deferred (most notably: only a
//! 63-entry hand-curated subset of the ~2231-entry named-character-
//! reference table, and no vendored WPT/html5lib-tests corpus yet).
//!
//! # Zero-dependency, `#![forbid(unsafe_code)]`
//!
//! This crate has no runtime dependency beyond `std`, and must never
//! depend on `command-model`/`command-bus`/`capability`/any protocol
//! crate — it sits below `machina-dom` (M2-T05) and `machina-native-core`
//! in the dependency graph. No `unsafe` anywhere.
//!
//! # Streaming contract
//!
//! [`Tokenizer`] is a pull-based state machine: `feed`/`finish` hand it
//! bytes, and [`Tokenizer::next_event`] drives processing one step at a
//! time, returning `Token`/`Diagnostic` events as they become available,
//! or `None` when more input is needed (or the tokenizer is finished).
//! Chunking input arbitrarily across `feed()` calls produces an equivalent
//! token stream to feeding it whole — see the crate's `tests/` for the
//! chunk-boundary equivalence property this guarantees, and
//! `TokenizerLimits::max_character_run_len` for the one sanctioned,
//! content-driven (not chunk-boundary-driven) exception.
//!
//! ```
//! use machina_html::{Token, Tokenizer, TokenizerEvent, TokenizerLimits};
//!
//! let mut tokenizer = Tokenizer::new(TokenizerLimits::default());
//! tokenizer.feed(b"<p>Hi &amp; bye</p>");
//! tokenizer.finish();
//!
//! let mut texts = Vec::new();
//! while let Some(event) = tokenizer.next_event() {
//!     if let TokenizerEvent::Token(Token::Character(c)) = event {
//!         texts.push(c.data);
//!     }
//! }
//! assert_eq!(texts.concat(), "Hi & bye");
//! ```

#![forbid(unsafe_code)]

mod diagnostics;
mod entities;
mod input;
mod limits;
mod state;
mod token;
mod tokenizer;

pub use diagnostics::{Diagnostic, ParseErrorCode, Severity};
pub use limits::{LimitKind, TokenizerLimits};
pub use token::{
    Attribute, CharacterToken, CommentToken, DoctypeToken, TagToken, TextContentState, Token,
    TokenizerEvent,
};
pub use tokenizer::Tokenizer;
