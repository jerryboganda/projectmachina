//! Byte-to-`char` decoding for the tokenizer's input stream (design §3).
//!
//! Handles the two chunk-boundary-sensitive normalization steps the
//! streaming equivalence contract depends on:
//! - **Partial UTF-8 at chunk boundaries**: an incomplete multi-byte
//!   sequence at the end of a `feed()` chunk is held (never dropped, never
//!   guessed at) until the next chunk supplies the rest. Genuinely invalid
//!   bytes become U+FFFD plus a diagnostic — never a panic, never an
//!   `unsafe` transmute.
//! - **Partial newlines** (WHATWG HTML §13.2.3.5): `\r\n` → `\n` and lone
//!   `\r` → `\n`, producing identical output regardless of where a `\r\n`
//!   pair was split across chunks.
//!
//! Decode diagnostics are queued **in-line** with the characters they
//! relate to (immediately before the U+FFFD they produced), not returned
//! out-of-band — the tokenizer is a lazy, pull-driven state machine
//! (`Tokenizer::next_event`), so a diagnostic must surface at the same
//! logical position in the stream as everything else, not eagerly at
//! `feed()` time (which could otherwise reorder it ahead of tokens for
//! earlier, not-yet-processed input).
//!
//! No `unsafe`, `unwrap`, `expect`, or unchecked cast — every branch here is
//! reachable from external byte input.

use std::collections::VecDeque;

use crate::diagnostics::{Diagnostic, ParseErrorCode, Severity};

/// One decoded character plus the stream-wide byte offset of its first byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PositionedChar {
    pub ch: char,
    pub pos: u64,
}

pub(crate) enum QueueItem {
    Char(PositionedChar),
    Diagnostic(Diagnostic),
}

pub(crate) struct InputStream {
    queue: VecDeque<QueueItem>,
    /// Up to 3 bytes of an incomplete UTF-8 sequence carried across a
    /// `feed()` boundary (a UTF-8 sequence is at most 4 bytes, so at most 3
    /// can ever be "waiting for more").
    carry: [u8; 3],
    carry_len: u8,
    /// Position of a held, not-yet-resolved `\r`, awaiting the next
    /// character to decide CRLF-vs-lone-CR normalization.
    pending_cr: Option<u64>,
    /// Cumulative byte count of everything ever passed to `feed()`.
    total_bytes_seen: u64,
    eof: bool,
}

impl InputStream {
    pub(crate) fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            carry: [0; 3],
            carry_len: 0,
            pending_cr: None,
            total_bytes_seen: 0,
            eof: false,
        }
    }

    /// Absolute byte position of the next byte that has not yet been
    /// reported to `feed()` (used to position the EOF pseudo-character).
    pub(crate) fn stream_end_pos(&self) -> u64 {
        self.total_bytes_seen
    }

    fn push_char(&mut self, ch: char, pos: u64) {
        if let Some(cr_pos) = self.pending_cr.take() {
            if ch == '\n' {
                // CRLF -> a single LF at the CR's own position.
                self.queue.push_back(QueueItem::Char(PositionedChar {
                    ch: '\n',
                    pos: cr_pos,
                }));
                return;
            }
            // Lone CR -> LF, then fall through to handle `ch` normally.
            self.queue.push_back(QueueItem::Char(PositionedChar {
                ch: '\n',
                pos: cr_pos,
            }));
        }
        if ch == '\r' {
            self.pending_cr = Some(pos);
            return;
        }
        self.queue
            .push_back(QueueItem::Char(PositionedChar { ch, pos }));
    }

    fn push_invalid_utf8(&mut self, pos: u64) {
        self.queue.push_back(QueueItem::Diagnostic(Diagnostic::new(
            ParseErrorCode::InvalidUtf8Sequence,
            pos,
            Severity::Error,
        )));
        self.push_char('\u{FFFD}', pos);
    }

    /// Decode `chunk`, appending newly available characters (and any
    /// invalid-byte-sequence diagnostics, in-line) to the internal queue.
    pub(crate) fn feed(&mut self, chunk: &[u8]) {
        if chunk.is_empty() {
            return;
        }

        let base_pos = self.total_bytes_seen - u64::from(self.carry_len);
        self.total_bytes_seen += chunk.len() as u64;

        let mut combined: Vec<u8> = Vec::with_capacity(self.carry_len as usize + chunk.len());
        combined.extend_from_slice(&self.carry[..self.carry_len as usize]);
        combined.extend_from_slice(chunk);
        self.carry_len = 0;

        let mut idx = 0usize;
        while idx < combined.len() {
            let slice = &combined[idx..];
            match std::str::from_utf8(slice) {
                Ok(valid) => {
                    for (offset, ch) in valid.char_indices() {
                        self.push_char(ch, base_pos + (idx + offset) as u64);
                    }
                    idx = combined.len();
                }
                Err(err) => {
                    let valid_up_to = err.valid_up_to();
                    if valid_up_to > 0 {
                        if let Ok(valid) = std::str::from_utf8(&slice[..valid_up_to]) {
                            for (offset, ch) in valid.char_indices() {
                                self.push_char(ch, base_pos + (idx + offset) as u64);
                            }
                        }
                    }
                    idx += valid_up_to;
                    match err.error_len() {
                        Some(bad_len) => {
                            let pos = base_pos + idx as u64;
                            self.push_invalid_utf8(pos);
                            idx += bad_len.max(1);
                        }
                        None => {
                            // Incomplete sequence at the very end of this
                            // chunk: hold it for the next `feed()`.
                            let remaining = &combined[idx..];
                            let len = remaining.len().min(self.carry.len());
                            self.carry[..len].copy_from_slice(&remaining[..len]);
                            self.carry_len = len as u8;
                            idx = combined.len();
                        }
                    }
                }
            }
        }
    }

    /// True end of stream: resolve any held lone `\r` and stop waiting for
    /// more bytes to complete a pending partial sequence. A still-pending
    /// partial UTF-8 sequence at true EOF is invalid input; it is reported
    /// once as U+FFFD, never silently dropped.
    pub(crate) fn finish(&mut self) {
        if self.carry_len > 0 {
            let pos = self.total_bytes_seen - u64::from(self.carry_len);
            self.push_invalid_utf8(pos);
            self.carry_len = 0;
        }
        if let Some(cr_pos) = self.pending_cr.take() {
            self.queue.push_back(QueueItem::Char(PositionedChar {
                ch: '\n',
                pos: cr_pos,
            }));
        }
        self.eof = true;
    }

    /// Pop the next queued item (character or diagnostic), if any is
    /// currently available.
    pub(crate) fn next_item(&mut self) -> Option<QueueItem> {
        self.queue.pop_front()
    }

    pub(crate) fn is_eof(&self) -> bool {
        self.eof
    }
}

impl Default for InputStream {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_all(chunks: &[&[u8]]) -> (Vec<char>, Vec<Diagnostic>) {
        let mut input = InputStream::new();
        let mut chars = Vec::new();
        let mut diags = Vec::new();
        let drain =
            |input: &mut InputStream, chars: &mut Vec<char>, diags: &mut Vec<Diagnostic>| {
                while let Some(item) = input.next_item() {
                    match item {
                        QueueItem::Char(pc) => chars.push(pc.ch),
                        QueueItem::Diagnostic(d) => diags.push(d),
                    }
                }
            };
        for chunk in chunks {
            input.feed(chunk);
            drain(&mut input, &mut chars, &mut diags);
        }
        input.finish();
        drain(&mut input, &mut chars, &mut diags);
        (chars, diags)
    }

    #[test]
    fn ascii_roundtrips() {
        let (chars, diags) = decode_all(&[b"hello"]);
        assert_eq!(chars, vec!['h', 'e', 'l', 'l', 'o']);
        assert!(diags.is_empty());
    }

    #[test]
    fn multibyte_utf8_split_across_chunks_decodes_identically_to_unsplit() {
        let whole = "héllo wörld \u{1F600}".as_bytes().to_vec();
        let (whole_chars, whole_diags) = decode_all(&[&whole]);
        assert!(whole_diags.is_empty());

        for split in 0..=whole.len() {
            let (a, b) = whole.split_at(split);
            let (chars, diags) = decode_all(&[a, b]);
            assert_eq!(chars, whole_chars, "split at byte {split}");
            assert!(diags.is_empty(), "split at byte {split}");
        }
    }

    #[test]
    fn crlf_split_across_chunks_normalizes_identically_to_unsplit() {
        let (whole, _) = decode_all(&[b"a\r\nb"]);
        let (split, _) = decode_all(&[b"a\r", b"\nb"]);
        assert_eq!(whole, vec!['a', '\n', 'b']);
        assert_eq!(split, whole);
    }

    #[test]
    fn lone_cr_at_true_eof_normalizes_to_lf() {
        let (chars, _) = decode_all(&[b"a\r"]);
        assert_eq!(chars, vec!['a', '\n']);
    }

    #[test]
    fn lone_cr_not_followed_by_lf_normalizes_to_lf() {
        let (chars, _) = decode_all(&[b"a\rb"]);
        assert_eq!(chars, vec!['a', '\n', 'b']);
    }

    #[test]
    fn invalid_byte_becomes_replacement_character_with_diagnostic() {
        let (chars, diags) = decode_all(&[&[b'a', 0xFF, b'b']]);
        assert_eq!(chars, vec!['a', '\u{FFFD}', 'b']);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, ParseErrorCode::InvalidUtf8Sequence);
    }

    #[test]
    fn truncated_multibyte_sequence_at_true_eof_becomes_replacement_character() {
        // 0xE2 0x82 is the first two bytes of a 3-byte sequence (e.g. €)
        // that never gets its third byte.
        let (chars, diags) = decode_all(&[&[b'a', 0xE2, 0x82]]);
        assert_eq!(chars, vec!['a', '\u{FFFD}']);
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn positions_are_stream_wide_across_feed_calls() {
        let mut input = InputStream::new();
        input.feed(b"ab");
        input.feed(b"cd");
        let mut positions = Vec::new();
        while let Some(QueueItem::Char(pc)) = input.next_item() {
            positions.push(pc.pos);
        }
        assert_eq!(positions, vec![0, 1, 2, 3]);
    }
}
