//! The tokenizer state machine driver (design §1, §3, §4).
//!
//! `Tokenizer::next_event` is the single non-recursive driver: it reads at
//! most one input character per internal `step()` call and returns control
//! to the caller the moment an event (token or diagnostic) is available, or
//! the moment input is exhausted and more is needed. All state-machine
//! working state — the in-progress tag/comment/doctype, the character-
//! reference scratch fields, the pending character-data run — lives in
//! `Tokenizer`'s own fields, never on the Rust call stack across
//! `feed()`/`finish()` boundaries. That is what makes chunked and
//! unchunked input produce an equivalent token stream "for free" (design
//! §3) rather than needing bespoke pause/resume logic, and what makes deep
//! adversarial nesting structurally unable to blow the stack (there is no
//! recursion here at all — unlike the tree builder, M2-T04, which needs an
//! explicit depth limit).
//!
//! **The external state-switch hook and pull-API timing**: `switch_to`
//! (design §7) must take effect before the tokenizer scans the byte
//! immediately following a `StartTag` token. `next_event` guarantees this:
//! a single `step()` call consumes at most one input character, and the
//! event queue is drained (`events.pop_front()`) *before* another
//! character is ever read. So a caller that calls `switch_to` right after
//! observing a `StartTag` via `next_event`, before calling `next_event`
//! again, is guaranteed the switch lands before the next byte is scanned.
//!
//! No `unsafe`, `unwrap`, `expect`, or unchecked cast anywhere in this
//! module — every branch here is reachable from external byte input.

use std::collections::VecDeque;

use crate::diagnostics::{Diagnostic, ParseErrorCode, Severity};
use crate::entities::{self, ENTITY_TABLE};
use crate::input::{InputStream, QueueItem};
use crate::limits::{LimitKind, TokenizerLimits};
use crate::state::State;
use crate::token::{
    Attribute, CharacterToken, CommentToken, DoctypeToken, TagToken, TextContentState, Token,
    TokenizerEvent,
};

#[derive(Clone, Copy, Debug, PartialEq)]
enum InputChar {
    Char(char, u64),
    Eof(u64),
}

impl InputChar {
    fn pos(self) -> u64 {
        match self {
            InputChar::Char(_, pos) | InputChar::Eof(pos) => pos,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum DoctypeField {
    Name,
    PublicId,
    SystemId,
}

fn is_html_whitespace(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\x0C' | ' ')
}

/// The streaming HTML tokenizer. See the crate root and design doc for the
/// full contract; `new`/`feed`/`finish`/`switch_to`/`last_start_tag_name`/
/// `next_event` are the entire public surface (design §7).
pub struct Tokenizer {
    state: State,
    return_state: State,
    limits: TokenizerLimits,
    input: InputStream,
    pushback: VecDeque<InputChar>,
    finished: bool,

    current_tag_name: String,
    current_tag_is_end: bool,
    current_tag_self_closing: bool,
    current_tag_attrs: Vec<Attribute>,
    current_attr_name: String,
    current_attr_value: String,
    current_attr_active: bool,
    current_attr_discard: bool,

    current_comment: String,
    current_doctype: DoctypeToken,

    /// General-purpose char trail used, one at a time (never concurrently),
    /// by: named-character-reference scanning, `<!--`/DOCTYPE/`[CDATA[`
    /// disambiguation in markup-declaration-open, the PUBLIC/SYSTEM keyword
    /// scan, and RCDATA/RAWTEXT/script-data end-tag-name literal-fallback
    /// reconstruction.
    scan_trail: Vec<InputChar>,
    /// Longest complete named-character-reference match found so far during
    /// a `NamedCharacterReference` scan: `(matched length in scan_trail,
    /// codepoint, legacy-no-semicolon-ok)`.
    scan_best_match: Option<(usize, u32, bool)>,
    named_char_ref_scan_steps: usize,
    /// Lowercase running comparison buffer for the script-data
    /// double-escape start/end "script" keyword check.
    temp_buffer: String,

    char_ref_code: u32,
    char_ref_is_hex: bool,
    char_ref_marker: char,

    last_start_tag_name: Option<String>,

    char_run: String,

    construct_start_pos: u64,
    tag_name_capped: bool,
    attributes_capped: bool,
    attr_value_capped: bool,
    comment_capped: bool,
    doctype_field_capped: bool,

    events: VecDeque<TokenizerEvent>,
    diagnostics_this_call: usize,
    too_many_diagnostics_this_call: bool,
}

impl Default for Tokenizer {
    fn default() -> Self {
        Self::new(TokenizerLimits::default())
    }
}

impl Tokenizer {
    pub fn new(limits: TokenizerLimits) -> Self {
        Self {
            state: State::Data,
            return_state: State::Data,
            limits,
            input: InputStream::new(),
            pushback: VecDeque::new(),
            finished: false,
            current_tag_name: String::new(),
            current_tag_is_end: false,
            current_tag_self_closing: false,
            current_tag_attrs: Vec::new(),
            current_attr_name: String::new(),
            current_attr_value: String::new(),
            current_attr_active: false,
            current_attr_discard: false,
            current_comment: String::new(),
            current_doctype: DoctypeToken::default(),
            scan_trail: Vec::new(),
            scan_best_match: None,
            named_char_ref_scan_steps: 0,
            temp_buffer: String::new(),
            char_ref_code: 0,
            char_ref_is_hex: false,
            char_ref_marker: 'x',
            last_start_tag_name: None,
            char_run: String::new(),
            construct_start_pos: 0,
            tag_name_capped: false,
            attributes_capped: false,
            attr_value_capped: false,
            comment_capped: false,
            doctype_field_capped: false,
            events: VecDeque::new(),
            diagnostics_this_call: 0,
            too_many_diagnostics_this_call: false,
        }
    }

    /// Feed the next chunk of bytes. Never blocks and never runs the state
    /// machine itself (design's pull model) — call `next_event` to drive
    /// processing and drain events.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.diagnostics_this_call = 0;
        self.too_many_diagnostics_this_call = false;
        self.input.feed(chunk);
    }

    /// Declare true end of stream. Subsequent `next_event` calls will drain
    /// remaining buffered input, then emit `Token::Eof` exactly once.
    pub fn finish(&mut self) {
        self.diagnostics_this_call = 0;
        self.too_many_diagnostics_this_call = false;
        self.input.finish();
    }

    /// External state-switch hook (design §7): the tree builder calls this
    /// immediately after observing a `StartTag` token for an element whose
    /// content model requires a non-`Data` text mode.
    pub fn switch_to(&mut self, state: TextContentState) {
        self.state = match state {
            TextContentState::Data => State::Data,
            TextContentState::Rcdata => State::Rcdata,
            TextContentState::Rawtext => State::Rawtext,
            TextContentState::ScriptData => State::ScriptData,
            TextContentState::Plaintext => State::Plaintext,
        };
    }

    /// The most recently emitted start tag's name, for "appropriate end
    /// tag token" matching (design §7).
    pub fn last_start_tag_name(&self) -> Option<&str> {
        self.last_start_tag_name.as_deref()
    }

    /// Pull the next token or diagnostic, driving the state machine as
    /// needed. Returns `None` when no more input is currently available
    /// (call `feed`/`finish` and try again) or once `Token::Eof` has been
    /// returned (the tokenizer is then permanently finished).
    pub fn next_event(&mut self) -> Option<TokenizerEvent> {
        loop {
            if let Some(event) = self.events.pop_front() {
                return Some(event);
            }
            if self.finished {
                return None;
            }
            let ic = self.read_char()?;
            self.step(ic);
        }
    }

    fn read_char(&mut self) -> Option<InputChar> {
        if let Some(ic) = self.pushback.pop_front() {
            return Some(ic);
        }
        loop {
            match self.input.next_item() {
                Some(QueueItem::Diagnostic(d)) => {
                    self.raise(d.code, d.position, d.severity);
                }
                Some(QueueItem::Char(pc)) => return Some(InputChar::Char(pc.ch, pc.pos)),
                None => {
                    if self.input.is_eof() {
                        return Some(InputChar::Eof(self.input.stream_end_pos()));
                    }
                    return None;
                }
            }
        }
    }

    fn unread_char(&mut self, ic: InputChar) {
        self.pushback.push_front(ic);
    }

    fn unread_sequence<I>(&mut self, items: I)
    where
        I: DoubleEndedIterator<Item = InputChar>,
    {
        for item in items.rev() {
            self.pushback.push_front(item);
        }
    }

    fn raise(&mut self, code: ParseErrorCode, pos: u64, severity: Severity) {
        if self.too_many_diagnostics_this_call {
            return;
        }
        self.diagnostics_this_call += 1;
        if self.diagnostics_this_call > self.limits.max_diagnostics_buffered {
            self.too_many_diagnostics_this_call = true;
            self.events
                .push_back(TokenizerEvent::Diagnostic(Diagnostic::new(
                    ParseErrorCode::TooManyDiagnostics,
                    pos,
                    Severity::Warning,
                )));
            return;
        }
        self.events
            .push_back(TokenizerEvent::Diagnostic(Diagnostic::new(
                code, pos, severity,
            )));
    }

    fn emit_non_character(&mut self, token: Token) {
        self.flush_char_run();
        self.events.push_back(TokenizerEvent::Token(token));
    }

    fn flush_char_run(&mut self) {
        if !self.char_run.is_empty() {
            let data = std::mem::take(&mut self.char_run);
            self.events
                .push_back(TokenizerEvent::Token(Token::Character(CharacterToken {
                    data,
                })));
        }
    }

    fn finalize_eof(&mut self) {
        self.emit_non_character(Token::Eof);
        self.finished = true;
    }

    fn eof_in_tag(&mut self, pos: u64) {
        self.raise(ParseErrorCode::EofInTag, pos, Severity::Error);
        self.finalize_eof();
    }

    fn eof_in_doctype(&mut self, pos: u64) {
        self.raise(ParseErrorCode::EofInDoctype, pos, Severity::Error);
        self.current_doctype.force_quirks = true;
        self.emit_doctype();
        self.finalize_eof();
    }

    fn eof_in_comment(&mut self, pos: u64) {
        self.raise(ParseErrorCode::EofInComment, pos, Severity::Error);
        self.emit_comment();
        self.finalize_eof();
    }

    // --- bounded-growth helpers (design §4) ---

    fn push_text_char(&mut self, c: char, pos: u64) {
        let c = if c == '\0' {
            self.raise(
                ParseErrorCode::UnexpectedNullCharacter,
                pos,
                Severity::Error,
            );
            '\u{FFFD}'
        } else {
            c
        };
        self.char_run.push(c);
        if self.char_run.len() >= self.limits.max_character_run_len {
            self.raise(
                ParseErrorCode::LimitExceeded(LimitKind::CharacterRunLen),
                pos,
                Severity::Warning,
            );
            self.flush_char_run();
        }
    }

    fn append_return_char(&mut self, c: char) {
        if self.return_state_is_attribute_value() {
            self.current_attr_value.push(c);
        } else {
            self.char_run.push(c);
        }
    }

    fn return_state_is_attribute_value(&self) -> bool {
        matches!(
            self.return_state,
            State::AttributeValueDoubleQuoted
                | State::AttributeValueSingleQuoted
                | State::AttributeValueUnquoted
        )
    }

    fn push_attr_value_char(&mut self, c: char, pos: u64) {
        let c = if c == '\0' {
            self.raise(
                ParseErrorCode::UnexpectedNullCharacter,
                pos,
                Severity::Error,
            );
            '\u{FFFD}'
        } else {
            c
        };
        if self.current_attr_value.len() >= self.limits.max_attribute_value_len {
            if !self.attr_value_capped {
                self.raise(
                    ParseErrorCode::LimitExceeded(LimitKind::AttributeValueLen),
                    pos,
                    Severity::Warning,
                );
                self.attr_value_capped = true;
            }
            return;
        }
        self.current_attr_value.push(c);
    }

    fn push_tag_name_char(&mut self, c: char, pos: u64) {
        let c = if c == '\0' {
            self.raise(
                ParseErrorCode::UnexpectedNullCharacter,
                pos,
                Severity::Error,
            );
            '\u{FFFD}'
        } else {
            c.to_ascii_lowercase()
        };
        if self.current_tag_name.len() >= self.limits.max_tag_name_len {
            if !self.tag_name_capped {
                self.raise(
                    ParseErrorCode::LimitExceeded(LimitKind::TagNameLen),
                    pos,
                    Severity::Warning,
                );
                self.tag_name_capped = true;
            }
            return;
        }
        self.current_tag_name.push(c);
    }

    fn push_attr_name_char(&mut self, c: char, pos: u64) {
        let c = if c == '\0' {
            self.raise(
                ParseErrorCode::UnexpectedNullCharacter,
                pos,
                Severity::Error,
            );
            '\u{FFFD}'
        } else {
            c.to_ascii_lowercase()
        };
        self.current_attr_name.push(c);
    }

    fn push_comment_char(&mut self, c: char, pos: u64) {
        let c = if c == '\0' {
            self.raise(
                ParseErrorCode::UnexpectedNullCharacter,
                pos,
                Severity::Error,
            );
            '\u{FFFD}'
        } else {
            c
        };
        if self.current_comment.len() >= self.limits.max_comment_len {
            if !self.comment_capped {
                self.raise(
                    ParseErrorCode::LimitExceeded(LimitKind::CommentLen),
                    pos,
                    Severity::Warning,
                );
                self.comment_capped = true;
            }
            return;
        }
        self.current_comment.push(c);
    }

    fn push_doctype_field_char(&mut self, field: DoctypeField, c: char, pos: u64) {
        let c = if c == '\0' {
            self.raise(
                ParseErrorCode::UnexpectedNullCharacter,
                pos,
                Severity::Error,
            );
            '\u{FFFD}'
        } else if matches!(field, DoctypeField::Name) {
            c.to_ascii_lowercase()
        } else {
            c
        };
        let current_len = match field {
            DoctypeField::Name => self.current_doctype.name.as_ref().map(String::len),
            DoctypeField::PublicId => self.current_doctype.public_id.as_ref().map(String::len),
            DoctypeField::SystemId => self.current_doctype.system_id.as_ref().map(String::len),
        }
        .unwrap_or(0);
        if current_len >= self.limits.max_doctype_field_len {
            if !self.doctype_field_capped {
                self.raise(
                    ParseErrorCode::LimitExceeded(LimitKind::DoctypeFieldLen),
                    pos,
                    Severity::Warning,
                );
                self.doctype_field_capped = true;
            }
            return;
        }
        let target = match field {
            DoctypeField::Name => &mut self.current_doctype.name,
            DoctypeField::PublicId => &mut self.current_doctype.public_id,
            DoctypeField::SystemId => &mut self.current_doctype.system_id,
        };
        target.get_or_insert_with(String::new).push(c);
    }

    fn reset_construct_buffers(&mut self) {
        self.current_tag_name.clear();
        self.current_tag_attrs.clear();
        self.current_attr_active = false;
        self.current_comment.clear();
        self.current_doctype = DoctypeToken::default();
        self.scan_trail.clear();
        self.temp_buffer.clear();
    }

    /// Hard backstop distinct from the per-field limits: force-terminates
    /// an in-progress tag/comment/doctype construct as bogus/oversized and
    /// resyncs to the data state. Returns `true` if it fired (caller must
    /// stop processing this char under the old state).
    fn check_and_handle_span_limit(&mut self, pos: u64) -> bool {
        if pos.saturating_sub(self.construct_start_pos)
            >= self.limits.max_tag_or_comment_byte_span as u64
        {
            self.raise(
                ParseErrorCode::LimitExceeded(LimitKind::TagOrCommentByteSpan),
                pos,
                Severity::Error,
            );
            self.reset_construct_buffers();
            self.state = State::Data;
            true
        } else {
            false
        }
    }

    // --- tag/attribute/comment/doctype token assembly ---

    fn begin_tag(&mut self, is_end: bool, pos: u64) {
        self.current_tag_name.clear();
        self.current_tag_is_end = is_end;
        self.current_tag_self_closing = false;
        self.current_tag_attrs.clear();
        self.current_attr_active = false;
        self.tag_name_capped = false;
        self.attributes_capped = false;
        self.construct_start_pos = pos;
    }

    fn begin_attribute(&mut self) {
        self.current_attr_name.clear();
        self.current_attr_value.clear();
        self.current_attr_active = true;
        self.current_attr_discard = false;
        self.attr_value_capped = false;
    }

    fn finish_attribute_name(&mut self, pos: u64) {
        if self
            .current_tag_attrs
            .iter()
            .any(|a| a.name == self.current_attr_name)
        {
            self.raise(ParseErrorCode::DuplicateAttribute, pos, Severity::Warning);
            self.current_attr_discard = true;
        }
    }

    fn finalize_attribute(&mut self, pos: u64) {
        if !self.current_attr_active {
            return;
        }
        self.current_attr_active = false;
        if self.current_attr_discard {
            return;
        }
        if self.current_tag_attrs.len() >= self.limits.max_attribute_count {
            if !self.attributes_capped {
                self.raise(
                    ParseErrorCode::LimitExceeded(LimitKind::AttributeCount),
                    pos,
                    Severity::Warning,
                );
                self.attributes_capped = true;
            }
            return;
        }
        self.current_tag_attrs.push(Attribute {
            name: std::mem::take(&mut self.current_attr_name),
            value: std::mem::take(&mut self.current_attr_value),
        });
    }

    fn emit_current_tag(&mut self, pos: u64) {
        if self.current_attr_active {
            self.finalize_attribute(pos);
        }
        let name = std::mem::take(&mut self.current_tag_name);
        let attributes = std::mem::take(&mut self.current_tag_attrs);
        let self_closing = self.current_tag_self_closing;
        let is_end = self.current_tag_is_end;
        self.current_tag_self_closing = false;
        self.current_tag_is_end = false;
        self.attributes_capped = false;
        self.tag_name_capped = false;
        if is_end {
            if !attributes.is_empty() {
                self.raise(ParseErrorCode::EndTagWithAttributes, pos, Severity::Warning);
            }
            if self_closing {
                self.raise(
                    ParseErrorCode::EndTagWithTrailingSolidus,
                    pos,
                    Severity::Warning,
                );
            }
            self.emit_non_character(Token::EndTag(TagToken {
                name,
                self_closing,
                attributes,
            }));
        } else {
            self.last_start_tag_name = Some(name.clone());
            self.emit_non_character(Token::StartTag(TagToken {
                name,
                self_closing,
                attributes,
            }));
        }
    }

    fn emit_comment(&mut self) {
        let data = std::mem::take(&mut self.current_comment);
        self.comment_capped = false;
        self.emit_non_character(Token::Comment(CommentToken { data }));
    }

    fn emit_doctype(&mut self) {
        let doctype = std::mem::take(&mut self.current_doctype);
        self.doctype_field_capped = false;
        self.emit_non_character(Token::Doctype(doctype));
    }

    fn is_appropriate_end_tag(&self) -> bool {
        match &self.last_start_tag_name {
            Some(name) => name == &self.current_tag_name,
            None => false,
        }
    }

    fn scan_trail_as_string(&self) -> String {
        self.scan_trail
            .iter()
            .filter_map(|item| match item {
                InputChar::Char(c, _) => Some(*c),
                InputChar::Eof(_) => None,
            })
            .collect()
    }

    fn flush_scan_trail_as_literal(&mut self) {
        self.append_return_char('&');
        let trail = std::mem::take(&mut self.scan_trail);
        for item in trail {
            if let InputChar::Char(c, _) = item {
                self.append_return_char(c);
            }
        }
    }

    fn flush_numeric_prefix_as_literal(&mut self) {
        self.append_return_char('&');
        self.append_return_char('#');
        if self.char_ref_is_hex {
            self.append_return_char(self.char_ref_marker);
        }
    }

    fn emit_char_ref_codepoint(&mut self, codepoint: u32) {
        let c = char::from_u32(codepoint).unwrap_or('\u{FFFD}');
        self.append_return_char(c);
    }

    // --- the driver ---

    fn step(&mut self, ic: InputChar) {
        let pos = ic.pos();
        match self.state {
            State::Data => self.step_data(ic, pos),
            State::Rcdata => self.step_rcdata(ic, pos),
            State::Rawtext => self.step_rawtext(ic, pos),
            State::ScriptData => self.step_script_data(ic, pos),
            State::Plaintext => self.step_plaintext(ic, pos),

            State::TagOpen => self.step_tag_open(ic, pos),
            State::EndTagOpen => self.step_end_tag_open(ic, pos),
            State::TagName => self.step_tag_name(ic, pos),

            State::RcdataLessThanSign => {
                self.step_text_less_than_sign(ic, pos, State::Rcdata, State::RcdataEndTagOpen)
            }
            State::RcdataEndTagOpen => {
                self.step_text_end_tag_open(ic, pos, State::Rcdata, State::RcdataEndTagName)
            }
            State::RcdataEndTagName => self.handle_text_end_tag_name(ic, State::Rcdata),

            State::RawtextLessThanSign => {
                self.step_text_less_than_sign(ic, pos, State::Rawtext, State::RawtextEndTagOpen)
            }
            State::RawtextEndTagOpen => {
                self.step_text_end_tag_open(ic, pos, State::Rawtext, State::RawtextEndTagName)
            }
            State::RawtextEndTagName => self.handle_text_end_tag_name(ic, State::Rawtext),

            State::ScriptDataLessThanSign => self.step_script_data_less_than_sign(ic, pos),
            State::ScriptDataEndTagOpen => {
                self.step_text_end_tag_open(ic, pos, State::ScriptData, State::ScriptDataEndTagName)
            }
            State::ScriptDataEndTagName => self.handle_text_end_tag_name(ic, State::ScriptData),
            State::ScriptDataEscapeStart => self.step_script_data_escape_start(ic, pos),
            State::ScriptDataEscapeStartDash => self.step_script_data_escape_start_dash(ic, pos),
            State::ScriptDataEscaped => self.step_script_data_escaped(ic, pos),
            State::ScriptDataEscapedDash => self.step_script_data_escaped_dash(ic, pos),
            State::ScriptDataEscapedDashDash => self.step_script_data_escaped_dash_dash(ic, pos),
            State::ScriptDataEscapedLessThanSign => {
                self.step_script_data_escaped_less_than_sign(ic, pos)
            }
            State::ScriptDataEscapedEndTagOpen => self.step_text_end_tag_open(
                ic,
                pos,
                State::ScriptDataEscaped,
                State::ScriptDataEscapedEndTagName,
            ),
            State::ScriptDataEscapedEndTagName => {
                self.handle_text_end_tag_name(ic, State::ScriptDataEscaped)
            }
            State::ScriptDataDoubleEscapeStart => self.step_double_escape_toggle(ic, pos, true),
            State::ScriptDataDoubleEscaped => self.step_script_data_double_escaped(ic, pos),
            State::ScriptDataDoubleEscapedDash => {
                self.step_script_data_double_escaped_dash(ic, pos)
            }
            State::ScriptDataDoubleEscapedDashDash => {
                self.step_script_data_double_escaped_dash_dash(ic, pos)
            }
            State::ScriptDataDoubleEscapedLessThanSign => {
                self.step_script_data_double_escaped_less_than_sign(ic, pos)
            }
            State::ScriptDataDoubleEscapeEnd => self.step_double_escape_toggle(ic, pos, false),

            State::BeforeAttributeName => self.step_before_attribute_name(ic, pos),
            State::AttributeName => self.step_attribute_name(ic, pos),
            State::AfterAttributeName => self.step_after_attribute_name(ic, pos),
            State::BeforeAttributeValue => self.step_before_attribute_value(ic, pos),
            State::AttributeValueDoubleQuoted => self.step_attribute_value_quoted(ic, pos, '"'),
            State::AttributeValueSingleQuoted => self.step_attribute_value_quoted(ic, pos, '\''),
            State::AttributeValueUnquoted => self.step_attribute_value_unquoted(ic, pos),
            State::AfterAttributeValueQuoted => self.step_after_attribute_value_quoted(ic, pos),
            State::SelfClosingStartTag => self.step_self_closing_start_tag(ic, pos),

            State::BogusComment => self.step_bogus_comment(ic, pos),
            State::MarkupDeclarationOpen => self.step_markup_declaration_open(ic),
            State::CommentStart => self.step_comment_start(ic, pos),
            State::CommentStartDash => self.step_comment_start_dash(ic, pos),
            State::Comment => self.step_comment(ic, pos),
            State::CommentLessThanSign => self.step_comment_less_than_sign(ic, pos),
            State::CommentLessThanSignBang => self.step_comment_less_than_sign_bang(ic, pos),
            State::CommentLessThanSignBangDash => {
                self.step_comment_less_than_sign_bang_dash(ic, pos)
            }
            State::CommentLessThanSignBangDashDash => {
                self.step_comment_less_than_sign_bang_dash_dash(ic, pos)
            }
            State::CommentEndDash => self.step_comment_end_dash(ic, pos),
            State::CommentEnd => self.step_comment_end(ic, pos),
            State::CommentEndBang => self.step_comment_end_bang(ic, pos),

            State::Doctype => self.step_doctype(ic, pos),
            State::BeforeDoctypeName => self.step_before_doctype_name(ic, pos),
            State::DoctypeName => self.step_doctype_name(ic, pos),
            State::AfterDoctypeName => self.step_after_doctype_name(ic),
            State::AfterDoctypePublicKeyword => self.step_after_doctype_public_keyword(ic, pos),
            State::BeforeDoctypePublicIdentifier => {
                self.step_before_doctype_public_identifier(ic, pos)
            }
            State::DoctypePublicIdentifierDoubleQuoted => self.step_doctype_identifier_quoted(
                ic,
                pos,
                DoctypeField::PublicId,
                '"',
                State::AfterDoctypePublicIdentifier,
            ),
            State::DoctypePublicIdentifierSingleQuoted => self.step_doctype_identifier_quoted(
                ic,
                pos,
                DoctypeField::PublicId,
                '\'',
                State::AfterDoctypePublicIdentifier,
            ),
            State::AfterDoctypePublicIdentifier => {
                self.step_after_doctype_public_identifier(ic, pos)
            }
            State::BetweenDoctypePublicAndSystemIdentifiers => {
                self.step_between_doctype_public_and_system_identifiers(ic, pos)
            }
            State::AfterDoctypeSystemKeyword => self.step_after_doctype_system_keyword(ic, pos),
            State::BeforeDoctypeSystemIdentifier => {
                self.step_before_doctype_system_identifier(ic, pos)
            }
            State::DoctypeSystemIdentifierDoubleQuoted => self.step_doctype_identifier_quoted(
                ic,
                pos,
                DoctypeField::SystemId,
                '"',
                State::AfterDoctypeSystemIdentifier,
            ),
            State::DoctypeSystemIdentifierSingleQuoted => self.step_doctype_identifier_quoted(
                ic,
                pos,
                DoctypeField::SystemId,
                '\'',
                State::AfterDoctypeSystemIdentifier,
            ),
            State::AfterDoctypeSystemIdentifier => {
                self.step_after_doctype_system_identifier(ic, pos)
            }
            State::BogusDoctype => self.step_bogus_doctype(ic, pos),

            State::CdataSection => self.step_cdata_section(ic, pos),
            State::CdataSectionBracket => self.step_cdata_section_bracket(ic, pos),
            State::CdataSectionEnd => self.step_cdata_section_end(ic, pos),

            State::CharacterReference => self.step_character_reference(ic),
            State::NamedCharacterReference => self.step_named_character_reference(ic),
            State::NumericCharacterReference => self.step_numeric_character_reference(ic),
            State::HexadecimalCharacterReferenceStart => self.step_hex_reference_start(ic, pos),
            State::DecimalCharacterReferenceStart => self.step_decimal_reference_start(ic, pos),
            State::HexadecimalCharacterReference => self.step_hex_reference(ic, pos),
            State::DecimalCharacterReference => self.step_decimal_reference(ic, pos),
            State::NumericCharacterReferenceEnd => self.step_numeric_reference_end(ic),
        }
    }

    // --- text-content modes ---

    fn step_data(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char('&', _) => {
                self.return_state = State::Data;
                self.scan_trail.clear();
                self.state = State::CharacterReference;
            }
            InputChar::Char('<', _) => {
                self.construct_start_pos = pos;
                self.state = State::TagOpen;
            }
            InputChar::Char(c, _) => self.push_text_char(c, pos),
            InputChar::Eof(_) => self.finalize_eof(),
        }
    }

    fn step_rcdata(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char('&', _) => {
                self.return_state = State::Rcdata;
                self.scan_trail.clear();
                self.state = State::CharacterReference;
            }
            InputChar::Char('<', _) => {
                self.construct_start_pos = pos;
                self.state = State::RcdataLessThanSign;
            }
            InputChar::Char(c, _) => self.push_text_char(c, pos),
            InputChar::Eof(_) => self.finalize_eof(),
        }
    }

    fn step_rawtext(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char('<', _) => {
                self.construct_start_pos = pos;
                self.state = State::RawtextLessThanSign;
            }
            InputChar::Char(c, _) => self.push_text_char(c, pos),
            InputChar::Eof(_) => self.finalize_eof(),
        }
    }

    fn step_script_data(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char('<', _) => {
                self.construct_start_pos = pos;
                self.state = State::ScriptDataLessThanSign;
            }
            InputChar::Char(c, _) => self.push_text_char(c, pos),
            InputChar::Eof(_) => self.finalize_eof(),
        }
    }

    fn step_plaintext(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char(c, _) => self.push_text_char(c, pos),
            InputChar::Eof(_) => self.finalize_eof(),
        }
    }

    // --- tag open / name ---

    fn step_tag_open(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char('!', _) => {
                self.scan_trail.clear();
                self.state = State::MarkupDeclarationOpen;
            }
            InputChar::Char('/', _) => self.state = State::EndTagOpen,
            InputChar::Char(c, _) if c.is_ascii_alphabetic() => {
                self.begin_tag(false, self.construct_start_pos);
                self.unread_char(ic);
                self.state = State::TagName;
            }
            InputChar::Char('?', _) => {
                self.raise(
                    ParseErrorCode::UnexpectedQuestionMarkInsteadOfTagName,
                    pos,
                    Severity::Error,
                );
                self.current_comment.clear();
                self.unread_char(ic);
                self.state = State::BogusComment;
            }
            InputChar::Eof(_) => {
                self.raise(ParseErrorCode::EofBeforeTagName, pos, Severity::Error);
                self.push_text_char('<', pos);
                self.finalize_eof();
            }
            InputChar::Char(_, _) => {
                self.raise(
                    ParseErrorCode::InvalidFirstCharacterOfTagName,
                    pos,
                    Severity::Error,
                );
                self.push_text_char('<', pos);
                self.unread_char(ic);
                self.state = State::Data;
            }
        }
    }

    fn step_end_tag_open(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char(c, _) if c.is_ascii_alphabetic() => {
                self.begin_tag(true, self.construct_start_pos);
                self.unread_char(ic);
                self.state = State::TagName;
            }
            InputChar::Char('>', _) => {
                self.raise(ParseErrorCode::MissingEndTagName, pos, Severity::Error);
                self.state = State::Data;
            }
            InputChar::Eof(_) => {
                self.raise(ParseErrorCode::EofBeforeTagName, pos, Severity::Error);
                self.push_text_char('<', pos);
                self.push_text_char('/', pos);
                self.finalize_eof();
            }
            InputChar::Char(_, _) => {
                self.raise(
                    ParseErrorCode::InvalidFirstCharacterOfTagName,
                    pos,
                    Severity::Error,
                );
                self.current_comment.clear();
                self.unread_char(ic);
                self.state = State::BogusComment;
            }
        }
    }

    fn step_tag_name(&mut self, ic: InputChar, pos: u64) {
        if self.check_and_handle_span_limit(pos) {
            self.unread_char(ic);
            return;
        }
        match ic {
            InputChar::Char(c, _) if is_html_whitespace(c) => {
                self.state = State::BeforeAttributeName
            }
            InputChar::Char('/', _) => self.state = State::SelfClosingStartTag,
            InputChar::Char('>', _) => {
                self.emit_current_tag(pos);
                self.state = State::Data;
            }
            InputChar::Eof(_) => self.eof_in_tag(pos),
            InputChar::Char(c, _) => self.push_tag_name_char(c, pos),
        }
    }

    // --- RCDATA/RAWTEXT/script-data-escaped end-tag recognition ---

    fn step_text_less_than_sign(
        &mut self,
        ic: InputChar,
        pos: u64,
        fallback: State,
        on_slash: State,
    ) {
        match ic {
            InputChar::Char('/', _) => {
                self.scan_trail.clear();
                self.current_tag_name.clear();
                self.state = on_slash;
            }
            _ => {
                self.push_text_char('<', pos);
                self.unread_char(ic);
                self.state = fallback;
            }
        }
    }

    fn step_text_end_tag_open(
        &mut self,
        ic: InputChar,
        pos: u64,
        fallback: State,
        on_alpha: State,
    ) {
        match ic {
            InputChar::Char(c, _) if c.is_ascii_alphabetic() => {
                self.current_tag_is_end = true;
                self.current_tag_self_closing = false;
                self.current_tag_attrs.clear();
                self.current_attr_active = false;
                self.unread_char(ic);
                self.state = on_alpha;
            }
            _ => {
                self.push_text_char('<', pos);
                self.push_text_char('/', pos);
                self.unread_char(ic);
                self.state = fallback;
            }
        }
    }

    fn handle_text_end_tag_name(&mut self, ic: InputChar, fallback_state: State) {
        if let InputChar::Char(c, _) = ic {
            if c.is_ascii_alphabetic() {
                self.current_tag_name.push(c.to_ascii_lowercase());
                self.scan_trail.push(ic);
                return;
            }
            let is_boundary = matches!(c, '\t' | '\n' | '\x0C' | ' ' | '/' | '>');
            if is_boundary && self.is_appropriate_end_tag() {
                if c == '/' {
                    self.state = State::SelfClosingStartTag;
                } else if c == '>' {
                    let pos = ic.pos();
                    self.emit_current_tag(pos);
                    self.state = State::Data;
                } else {
                    self.state = State::BeforeAttributeName;
                }
                return;
            }
        }
        self.push_text_char('<', self.construct_start_pos);
        self.push_text_char('/', self.construct_start_pos);
        let trail = std::mem::take(&mut self.scan_trail);
        for item in trail {
            if let InputChar::Char(c, p) = item {
                self.push_text_char(c, p);
            }
        }
        self.current_tag_name.clear();
        self.unread_char(ic);
        self.state = fallback_state;
    }

    // --- script data (plain + escaping subgroup) ---

    fn step_script_data_less_than_sign(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char('/', _) => {
                self.scan_trail.clear();
                self.current_tag_name.clear();
                self.state = State::ScriptDataEndTagOpen;
            }
            InputChar::Char('!', _) => {
                self.push_text_char('<', pos);
                self.push_text_char('!', pos);
                self.state = State::ScriptDataEscapeStart;
            }
            _ => {
                self.push_text_char('<', pos);
                self.unread_char(ic);
                self.state = State::ScriptData;
            }
        }
    }

    fn step_script_data_escape_start(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char('-', _) => {
                self.push_text_char('-', pos);
                self.state = State::ScriptDataEscapeStartDash;
            }
            _ => {
                self.unread_char(ic);
                self.state = State::ScriptData;
            }
        }
    }

    fn step_script_data_escape_start_dash(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char('-', _) => {
                self.push_text_char('-', pos);
                self.state = State::ScriptDataEscapedDashDash;
            }
            _ => {
                self.unread_char(ic);
                self.state = State::ScriptData;
            }
        }
    }

    fn step_script_data_escaped(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char('-', _) => {
                self.push_text_char('-', pos);
                self.state = State::ScriptDataEscapedDash;
            }
            InputChar::Char('<', _) => self.state = State::ScriptDataEscapedLessThanSign,
            InputChar::Char(c, _) => self.push_text_char(c, pos),
            InputChar::Eof(_) => {
                self.raise(
                    ParseErrorCode::EofInScriptHtmlCommentLikeText,
                    pos,
                    Severity::Error,
                );
                self.finalize_eof();
            }
        }
    }

    fn step_script_data_escaped_dash(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char('-', _) => {
                self.push_text_char('-', pos);
                self.state = State::ScriptDataEscapedDashDash;
            }
            InputChar::Char('<', _) => self.state = State::ScriptDataEscapedLessThanSign,
            InputChar::Char(c, _) => {
                self.push_text_char(c, pos);
                self.state = State::ScriptDataEscaped;
            }
            InputChar::Eof(_) => {
                self.raise(
                    ParseErrorCode::EofInScriptHtmlCommentLikeText,
                    pos,
                    Severity::Error,
                );
                self.finalize_eof();
            }
        }
    }

    fn step_script_data_escaped_dash_dash(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char('-', _) => self.push_text_char('-', pos),
            InputChar::Char('<', _) => self.state = State::ScriptDataEscapedLessThanSign,
            InputChar::Char('>', _) => {
                self.push_text_char('>', pos);
                self.state = State::ScriptData;
            }
            InputChar::Char(c, _) => {
                self.push_text_char(c, pos);
                self.state = State::ScriptDataEscaped;
            }
            InputChar::Eof(_) => {
                self.raise(
                    ParseErrorCode::EofInScriptHtmlCommentLikeText,
                    pos,
                    Severity::Error,
                );
                self.finalize_eof();
            }
        }
    }

    fn step_script_data_escaped_less_than_sign(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char('/', _) => {
                self.scan_trail.clear();
                self.current_tag_name.clear();
                self.state = State::ScriptDataEscapedEndTagOpen;
            }
            InputChar::Char(c, _) if c.is_ascii_alphabetic() => {
                self.temp_buffer.clear();
                self.push_text_char('<', pos);
                self.unread_char(ic);
                self.state = State::ScriptDataDoubleEscapeStart;
            }
            _ => {
                self.push_text_char('<', pos);
                self.unread_char(ic);
                self.state = State::ScriptDataEscaped;
            }
        }
    }

    fn step_double_escape_toggle(&mut self, ic: InputChar, pos: u64, entering_double: bool) {
        match ic {
            InputChar::Char(c, _) if c.is_ascii_alphabetic() => {
                self.temp_buffer.push(c.to_ascii_lowercase());
                self.push_text_char(c, pos);
            }
            InputChar::Char(c, _) if matches!(c, '\t' | '\n' | '\x0C' | ' ' | '/' | '>') => {
                self.push_text_char(c, pos);
                let matched_script = self.temp_buffer == "script";
                self.state = if matched_script == entering_double {
                    State::ScriptDataDoubleEscaped
                } else {
                    State::ScriptDataEscaped
                };
            }
            _ => {
                self.unread_char(ic);
                self.state = if entering_double {
                    State::ScriptDataEscaped
                } else {
                    State::ScriptDataDoubleEscaped
                };
            }
        }
    }

    fn step_script_data_double_escaped(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char('-', _) => {
                self.push_text_char('-', pos);
                self.state = State::ScriptDataDoubleEscapedDash;
            }
            InputChar::Char('<', _) => {
                self.push_text_char('<', pos);
                self.state = State::ScriptDataDoubleEscapedLessThanSign;
            }
            InputChar::Char(c, _) => self.push_text_char(c, pos),
            InputChar::Eof(_) => {
                self.raise(
                    ParseErrorCode::EofInScriptHtmlCommentLikeText,
                    pos,
                    Severity::Error,
                );
                self.finalize_eof();
            }
        }
    }

    fn step_script_data_double_escaped_dash(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char('-', _) => {
                self.push_text_char('-', pos);
                self.state = State::ScriptDataDoubleEscapedDashDash;
            }
            InputChar::Char('<', _) => {
                self.push_text_char('<', pos);
                self.state = State::ScriptDataDoubleEscapedLessThanSign;
            }
            InputChar::Char(c, _) => {
                self.push_text_char(c, pos);
                self.state = State::ScriptDataDoubleEscaped;
            }
            InputChar::Eof(_) => {
                self.raise(
                    ParseErrorCode::EofInScriptHtmlCommentLikeText,
                    pos,
                    Severity::Error,
                );
                self.finalize_eof();
            }
        }
    }

    fn step_script_data_double_escaped_dash_dash(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char('-', _) => self.push_text_char('-', pos),
            InputChar::Char('<', _) => {
                self.push_text_char('<', pos);
                self.state = State::ScriptDataDoubleEscapedLessThanSign;
            }
            InputChar::Char('>', _) => {
                self.push_text_char('>', pos);
                self.state = State::ScriptData;
            }
            InputChar::Char(c, _) => {
                self.push_text_char(c, pos);
                self.state = State::ScriptDataDoubleEscaped;
            }
            InputChar::Eof(_) => {
                self.raise(
                    ParseErrorCode::EofInScriptHtmlCommentLikeText,
                    pos,
                    Severity::Error,
                );
                self.finalize_eof();
            }
        }
    }

    fn step_script_data_double_escaped_less_than_sign(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char('/', _) => {
                self.temp_buffer.clear();
                self.push_text_char('/', pos);
                self.state = State::ScriptDataDoubleEscapeEnd;
            }
            _ => {
                self.unread_char(ic);
                self.state = State::ScriptDataDoubleEscaped;
            }
        }
    }

    // --- attributes ---

    fn step_before_attribute_name(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char(c, _) if is_html_whitespace(c) => {}
            InputChar::Char('/', _) | InputChar::Char('>', _) => {
                self.unread_char(ic);
                self.state = State::AfterAttributeName;
            }
            InputChar::Char('=', _) => {
                self.raise(
                    ParseErrorCode::UnexpectedEqualsSignBeforeAttributeName,
                    pos,
                    Severity::Error,
                );
                self.begin_attribute();
                self.current_attr_name.push('=');
                self.state = State::AttributeName;
            }
            InputChar::Eof(_) => self.eof_in_tag(pos),
            InputChar::Char(_, _) => {
                self.begin_attribute();
                self.unread_char(ic);
                self.state = State::AttributeName;
            }
        }
    }

    fn step_attribute_name(&mut self, ic: InputChar, pos: u64) {
        if self.check_and_handle_span_limit(pos) {
            self.unread_char(ic);
            return;
        }
        match ic {
            InputChar::Char(c, _) if is_html_whitespace(c) => {
                self.finish_attribute_name(pos);
                self.finalize_attribute(pos);
                self.state = State::AfterAttributeName;
            }
            InputChar::Char('/', _) => {
                self.finish_attribute_name(pos);
                self.finalize_attribute(pos);
                self.state = State::SelfClosingStartTag;
            }
            InputChar::Char('>', _) => {
                self.finish_attribute_name(pos);
                self.finalize_attribute(pos);
                self.emit_current_tag(pos);
                self.state = State::Data;
            }
            InputChar::Char('=', _) => {
                self.finish_attribute_name(pos);
                self.state = State::BeforeAttributeValue;
            }
            InputChar::Eof(_) => self.eof_in_tag(pos),
            InputChar::Char(c, _) if matches!(c, '"' | '\'' | '<') => {
                self.raise(
                    ParseErrorCode::UnexpectedCharacterInAttributeName,
                    pos,
                    Severity::Warning,
                );
                self.push_attr_name_char(c, pos);
            }
            InputChar::Char(c, _) => self.push_attr_name_char(c, pos),
        }
    }

    fn step_after_attribute_name(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char(c, _) if is_html_whitespace(c) => {}
            InputChar::Char('/', _) => self.state = State::SelfClosingStartTag,
            InputChar::Char('=', _) => self.state = State::BeforeAttributeValue,
            InputChar::Char('>', _) => {
                self.emit_current_tag(pos);
                self.state = State::Data;
            }
            InputChar::Eof(_) => self.eof_in_tag(pos),
            InputChar::Char(_, _) => {
                self.begin_attribute();
                self.unread_char(ic);
                self.state = State::AttributeName;
            }
        }
    }

    fn step_before_attribute_value(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char(c, _) if is_html_whitespace(c) => {}
            InputChar::Char('"', _) => self.state = State::AttributeValueDoubleQuoted,
            InputChar::Char('\'', _) => self.state = State::AttributeValueSingleQuoted,
            InputChar::Char('>', _) => {
                self.raise(ParseErrorCode::MissingAttributeValue, pos, Severity::Error);
                self.finalize_attribute(pos);
                self.emit_current_tag(pos);
                self.state = State::Data;
            }
            _ => {
                self.unread_char(ic);
                self.state = State::AttributeValueUnquoted;
            }
        }
    }

    fn step_attribute_value_quoted(&mut self, ic: InputChar, pos: u64, quote: char) {
        if self.check_and_handle_span_limit(pos) {
            self.unread_char(ic);
            return;
        }
        match ic {
            InputChar::Char(c, _) if c == quote => {
                self.finalize_attribute(pos);
                self.state = State::AfterAttributeValueQuoted;
            }
            InputChar::Char('&', _) => {
                self.return_state = self.state;
                self.scan_trail.clear();
                self.state = State::CharacterReference;
            }
            InputChar::Eof(_) => self.eof_in_tag(pos),
            InputChar::Char(c, _) => self.push_attr_value_char(c, pos),
        }
    }

    fn step_attribute_value_unquoted(&mut self, ic: InputChar, pos: u64) {
        if self.check_and_handle_span_limit(pos) {
            self.unread_char(ic);
            return;
        }
        match ic {
            InputChar::Char(c, _) if is_html_whitespace(c) => {
                self.finalize_attribute(pos);
                self.state = State::BeforeAttributeName;
            }
            InputChar::Char('&', _) => {
                self.return_state = State::AttributeValueUnquoted;
                self.scan_trail.clear();
                self.state = State::CharacterReference;
            }
            InputChar::Char('>', _) => {
                self.finalize_attribute(pos);
                self.emit_current_tag(pos);
                self.state = State::Data;
            }
            InputChar::Eof(_) => self.eof_in_tag(pos),
            InputChar::Char(c, _) if matches!(c, '"' | '\'' | '<' | '=' | '`') => {
                self.raise(
                    ParseErrorCode::UnexpectedCharacterInUnquotedAttributeValue,
                    pos,
                    Severity::Warning,
                );
                self.push_attr_value_char(c, pos);
            }
            InputChar::Char(c, _) => self.push_attr_value_char(c, pos),
        }
    }

    fn step_after_attribute_value_quoted(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char(c, _) if is_html_whitespace(c) => {
                self.state = State::BeforeAttributeName
            }
            InputChar::Char('/', _) => self.state = State::SelfClosingStartTag,
            InputChar::Char('>', _) => {
                self.emit_current_tag(pos);
                self.state = State::Data;
            }
            InputChar::Eof(_) => self.eof_in_tag(pos),
            InputChar::Char(_, _) => {
                self.raise(
                    ParseErrorCode::MissingWhitespaceBetweenDoctypePublicAndSystemIdentifiers,
                    pos,
                    Severity::Warning,
                );
                self.unread_char(ic);
                self.state = State::BeforeAttributeName;
            }
        }
    }

    fn step_self_closing_start_tag(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char('>', _) => {
                self.current_tag_self_closing = true;
                self.emit_current_tag(pos);
                self.state = State::Data;
            }
            InputChar::Eof(_) => self.eof_in_tag(pos),
            InputChar::Char(_, _) => {
                self.raise(
                    ParseErrorCode::UnexpectedSolidusInTag,
                    pos,
                    Severity::Warning,
                );
                self.unread_char(ic);
                self.state = State::BeforeAttributeName;
            }
        }
    }

    // --- comments / markup declaration ---

    fn step_bogus_comment(&mut self, ic: InputChar, pos: u64) {
        if self.check_and_handle_span_limit(pos) {
            self.unread_char(ic);
            return;
        }
        match ic {
            InputChar::Char('>', _) => {
                self.emit_comment();
                self.state = State::Data;
            }
            InputChar::Eof(_) => {
                self.emit_comment();
                self.finalize_eof();
            }
            InputChar::Char(c, _) => self.push_comment_char(c, pos),
        }
    }

    fn step_markup_declaration_open(&mut self, ic: InputChar) {
        if matches!(ic, InputChar::Char(..)) {
            self.scan_trail.push(ic);
        }
        let scanned = self.scan_trail_as_string();
        if scanned == "--" {
            self.current_comment.clear();
            self.construct_start_pos = ic.pos();
            self.state = State::CommentStart;
            self.scan_trail.clear();
            return;
        }
        if scanned.eq_ignore_ascii_case("doctype") {
            self.construct_start_pos = ic.pos();
            self.state = State::Doctype;
            self.scan_trail.clear();
            return;
        }
        if scanned == "[CDATA[" {
            self.state = State::CdataSection;
            self.scan_trail.clear();
            return;
        }
        let scanned_upper = scanned.to_ascii_uppercase();
        let could_extend = "--".starts_with(scanned.as_str())
            || "DOCTYPE".starts_with(scanned_upper.as_str())
            || "[CDATA[".starts_with(scanned.as_str());
        if could_extend && matches!(ic, InputChar::Char(..)) {
            return;
        }
        self.raise(
            ParseErrorCode::IncorrectlyOpenedComment,
            ic.pos(),
            Severity::Error,
        );
        self.current_comment.clear();
        let trail = std::mem::take(&mut self.scan_trail);
        self.unread_sequence(trail.into_iter());
        self.state = State::BogusComment;
    }

    fn step_comment_start(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char('-', _) => self.state = State::CommentStartDash,
            InputChar::Char('>', _) => {
                self.raise(
                    ParseErrorCode::AbruptClosingOfEmptyComment,
                    pos,
                    Severity::Error,
                );
                self.emit_comment();
                self.state = State::Data;
            }
            _ => {
                self.unread_char(ic);
                self.state = State::Comment;
            }
        }
    }

    fn step_comment_start_dash(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char('-', _) => self.state = State::CommentEnd,
            InputChar::Char('>', _) => {
                self.raise(
                    ParseErrorCode::AbruptClosingOfEmptyComment,
                    pos,
                    Severity::Error,
                );
                self.emit_comment();
                self.state = State::Data;
            }
            InputChar::Eof(_) => self.eof_in_comment(pos),
            _ => {
                self.push_comment_char('-', pos);
                self.unread_char(ic);
                self.state = State::Comment;
            }
        }
    }

    fn step_comment(&mut self, ic: InputChar, pos: u64) {
        if self.check_and_handle_span_limit(pos) {
            self.unread_char(ic);
            return;
        }
        match ic {
            InputChar::Char('<', _) => {
                self.push_comment_char('<', pos);
                self.state = State::CommentLessThanSign;
            }
            InputChar::Char('-', _) => self.state = State::CommentEndDash,
            InputChar::Eof(_) => self.eof_in_comment(pos),
            InputChar::Char(c, _) => self.push_comment_char(c, pos),
        }
    }

    fn step_comment_less_than_sign(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char('!', _) => {
                self.push_comment_char('!', pos);
                self.state = State::CommentLessThanSignBang;
            }
            InputChar::Char('<', _) => self.push_comment_char('<', pos),
            _ => {
                self.unread_char(ic);
                self.state = State::Comment;
            }
        }
    }

    fn step_comment_less_than_sign_bang(&mut self, ic: InputChar, _pos: u64) {
        match ic {
            InputChar::Char('-', _) => self.state = State::CommentLessThanSignBangDash,
            _ => {
                self.unread_char(ic);
                self.state = State::Comment;
            }
        }
    }

    fn step_comment_less_than_sign_bang_dash(&mut self, ic: InputChar, _pos: u64) {
        match ic {
            InputChar::Char('-', _) => self.state = State::CommentLessThanSignBangDashDash,
            _ => {
                self.unread_char(ic);
                self.state = State::CommentEndDash;
            }
        }
    }

    fn step_comment_less_than_sign_bang_dash_dash(&mut self, ic: InputChar, _pos: u64) {
        if !matches!(ic, InputChar::Char('>', _)) {
            self.raise(ParseErrorCode::NestedComment, ic.pos(), Severity::Warning);
        }
        self.unread_char(ic);
        self.state = State::CommentEnd;
    }

    fn step_comment_end_dash(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char('-', _) => self.state = State::CommentEnd,
            InputChar::Eof(_) => self.eof_in_comment(pos),
            _ => {
                self.push_comment_char('-', pos);
                self.unread_char(ic);
                self.state = State::Comment;
            }
        }
    }

    fn step_comment_end(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char('>', _) => {
                self.emit_comment();
                self.state = State::Data;
            }
            InputChar::Char('!', _) => self.state = State::CommentEndBang,
            InputChar::Char('-', _) => self.push_comment_char('-', pos),
            InputChar::Eof(_) => self.eof_in_comment(pos),
            _ => {
                self.push_comment_char('-', pos);
                self.push_comment_char('-', pos);
                self.unread_char(ic);
                self.state = State::Comment;
            }
        }
    }

    fn step_comment_end_bang(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char('-', _) => {
                self.push_comment_char('-', pos);
                self.push_comment_char('-', pos);
                self.push_comment_char('!', pos);
                self.state = State::CommentEndDash;
            }
            InputChar::Char('>', _) => {
                self.raise(
                    ParseErrorCode::IncorrectlyClosedComment,
                    pos,
                    Severity::Error,
                );
                self.emit_comment();
                self.state = State::Data;
            }
            InputChar::Eof(_) => self.eof_in_comment(pos),
            _ => {
                self.push_comment_char('-', pos);
                self.push_comment_char('-', pos);
                self.push_comment_char('!', pos);
                self.unread_char(ic);
                self.state = State::Comment;
            }
        }
    }

    // --- DOCTYPE ---

    fn step_doctype(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char(c, _) if is_html_whitespace(c) => self.state = State::BeforeDoctypeName,
            InputChar::Char('>', _) => {
                self.unread_char(ic);
                self.state = State::BeforeDoctypeName;
            }
            InputChar::Eof(_) => self.eof_in_doctype(pos),
            InputChar::Char(_, _) => {
                self.raise(
                    ParseErrorCode::MissingWhitespaceBeforeDoctypeName,
                    pos,
                    Severity::Warning,
                );
                self.unread_char(ic);
                self.state = State::BeforeDoctypeName;
            }
        }
    }

    fn step_before_doctype_name(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char(c, _) if is_html_whitespace(c) => {}
            InputChar::Char('>', _) => {
                self.raise(ParseErrorCode::MissingDoctypeName, pos, Severity::Error);
                self.current_doctype.force_quirks = true;
                self.emit_doctype();
                self.state = State::Data;
            }
            InputChar::Eof(_) => self.eof_in_doctype(pos),
            InputChar::Char(c, _) => {
                self.push_doctype_field_char(DoctypeField::Name, c, pos);
                self.state = State::DoctypeName;
            }
        }
    }

    fn step_doctype_name(&mut self, ic: InputChar, pos: u64) {
        if self.check_and_handle_span_limit(pos) {
            self.unread_char(ic);
            return;
        }
        match ic {
            InputChar::Char(c, _) if is_html_whitespace(c) => self.state = State::AfterDoctypeName,
            InputChar::Char('>', _) => {
                self.emit_doctype();
                self.state = State::Data;
            }
            InputChar::Eof(_) => self.eof_in_doctype(pos),
            InputChar::Char(c, _) => self.push_doctype_field_char(DoctypeField::Name, c, pos),
        }
    }

    fn step_after_doctype_name(&mut self, ic: InputChar) {
        let pos = ic.pos();
        match ic {
            InputChar::Char(c, _) if is_html_whitespace(c) => {}
            InputChar::Char('>', _) => {
                self.emit_doctype();
                self.state = State::Data;
            }
            InputChar::Eof(_) => self.eof_in_doctype(pos),
            InputChar::Char(_, _) => {
                self.scan_trail.push(ic);
                let scanned_upper = self.scan_trail_as_string().to_ascii_uppercase();
                if scanned_upper == "PUBLIC" {
                    self.scan_trail.clear();
                    self.state = State::AfterDoctypePublicKeyword;
                    return;
                }
                if scanned_upper == "SYSTEM" {
                    self.scan_trail.clear();
                    self.state = State::AfterDoctypeSystemKeyword;
                    return;
                }
                let could_extend = "PUBLIC".starts_with(scanned_upper.as_str())
                    || "SYSTEM".starts_with(scanned_upper.as_str());
                if could_extend {
                    return;
                }
                self.raise(
                    ParseErrorCode::InvalidCharacterSequenceAfterDoctypeName,
                    pos,
                    Severity::Error,
                );
                self.current_doctype.force_quirks = true;
                let trail = std::mem::take(&mut self.scan_trail);
                self.unread_sequence(trail.into_iter());
                self.state = State::BogusDoctype;
            }
        }
    }

    fn step_after_doctype_public_keyword(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char(c, _) if is_html_whitespace(c) => {
                self.state = State::BeforeDoctypePublicIdentifier
            }
            InputChar::Char('"', _) => {
                self.raise(
                    ParseErrorCode::MissingWhitespaceAfterDoctypePublicKeyword,
                    pos,
                    Severity::Warning,
                );
                self.current_doctype.public_id = Some(String::new());
                self.state = State::DoctypePublicIdentifierDoubleQuoted;
            }
            InputChar::Char('\'', _) => {
                self.raise(
                    ParseErrorCode::MissingWhitespaceAfterDoctypePublicKeyword,
                    pos,
                    Severity::Warning,
                );
                self.current_doctype.public_id = Some(String::new());
                self.state = State::DoctypePublicIdentifierSingleQuoted;
            }
            InputChar::Char('>', _) => {
                self.raise(
                    ParseErrorCode::MissingDoctypePublicIdentifier,
                    pos,
                    Severity::Error,
                );
                self.current_doctype.force_quirks = true;
                self.emit_doctype();
                self.state = State::Data;
            }
            InputChar::Eof(_) => self.eof_in_doctype(pos),
            InputChar::Char(_, _) => {
                self.raise(
                    ParseErrorCode::MissingQuoteBeforeDoctypePublicIdentifier,
                    pos,
                    Severity::Error,
                );
                self.current_doctype.force_quirks = true;
                self.unread_char(ic);
                self.state = State::BogusDoctype;
            }
        }
    }

    fn step_before_doctype_public_identifier(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char(c, _) if is_html_whitespace(c) => {}
            InputChar::Char('"', _) => {
                self.current_doctype.public_id = Some(String::new());
                self.state = State::DoctypePublicIdentifierDoubleQuoted;
            }
            InputChar::Char('\'', _) => {
                self.current_doctype.public_id = Some(String::new());
                self.state = State::DoctypePublicIdentifierSingleQuoted;
            }
            InputChar::Char('>', _) => {
                self.raise(
                    ParseErrorCode::MissingDoctypePublicIdentifier,
                    pos,
                    Severity::Error,
                );
                self.current_doctype.force_quirks = true;
                self.emit_doctype();
                self.state = State::Data;
            }
            InputChar::Eof(_) => self.eof_in_doctype(pos),
            InputChar::Char(_, _) => {
                self.raise(
                    ParseErrorCode::MissingQuoteBeforeDoctypePublicIdentifier,
                    pos,
                    Severity::Error,
                );
                self.current_doctype.force_quirks = true;
                self.unread_char(ic);
                self.state = State::BogusDoctype;
            }
        }
    }

    fn step_doctype_identifier_quoted(
        &mut self,
        ic: InputChar,
        pos: u64,
        field: DoctypeField,
        quote: char,
        next_state: State,
    ) {
        if self.check_and_handle_span_limit(pos) {
            self.unread_char(ic);
            return;
        }
        match ic {
            InputChar::Char(c, _) if c == quote => self.state = next_state,
            InputChar::Char('>', _) => {
                let code = match field {
                    DoctypeField::PublicId => ParseErrorCode::AbruptDoctypePublicIdentifier,
                    _ => ParseErrorCode::AbruptDoctypeSystemIdentifier,
                };
                self.raise(code, pos, Severity::Error);
                self.current_doctype.force_quirks = true;
                self.emit_doctype();
                self.state = State::Data;
            }
            InputChar::Eof(_) => self.eof_in_doctype(pos),
            InputChar::Char(c, _) => self.push_doctype_field_char(field, c, pos),
        }
    }

    fn step_after_doctype_public_identifier(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char(c, _) if is_html_whitespace(c) => {
                self.state = State::BetweenDoctypePublicAndSystemIdentifiers;
            }
            InputChar::Char('>', _) => {
                self.emit_doctype();
                self.state = State::Data;
            }
            InputChar::Char('"', _) => {
                self.raise(
                    ParseErrorCode::MissingWhitespaceBetweenDoctypePublicAndSystemIdentifiers,
                    pos,
                    Severity::Warning,
                );
                self.current_doctype.system_id = Some(String::new());
                self.state = State::DoctypeSystemIdentifierDoubleQuoted;
            }
            InputChar::Char('\'', _) => {
                self.raise(
                    ParseErrorCode::MissingWhitespaceBetweenDoctypePublicAndSystemIdentifiers,
                    pos,
                    Severity::Warning,
                );
                self.current_doctype.system_id = Some(String::new());
                self.state = State::DoctypeSystemIdentifierSingleQuoted;
            }
            InputChar::Eof(_) => self.eof_in_doctype(pos),
            InputChar::Char(_, _) => {
                self.raise(
                    ParseErrorCode::MissingQuoteBeforeDoctypeSystemIdentifier,
                    pos,
                    Severity::Error,
                );
                self.current_doctype.force_quirks = true;
                self.unread_char(ic);
                self.state = State::BogusDoctype;
            }
        }
    }

    fn step_between_doctype_public_and_system_identifiers(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char(c, _) if is_html_whitespace(c) => {}
            InputChar::Char('>', _) => {
                self.emit_doctype();
                self.state = State::Data;
            }
            InputChar::Char('"', _) => {
                self.current_doctype.system_id = Some(String::new());
                self.state = State::DoctypeSystemIdentifierDoubleQuoted;
            }
            InputChar::Char('\'', _) => {
                self.current_doctype.system_id = Some(String::new());
                self.state = State::DoctypeSystemIdentifierSingleQuoted;
            }
            InputChar::Eof(_) => self.eof_in_doctype(pos),
            InputChar::Char(_, _) => {
                self.raise(
                    ParseErrorCode::MissingQuoteBeforeDoctypeSystemIdentifier,
                    pos,
                    Severity::Error,
                );
                self.current_doctype.force_quirks = true;
                self.unread_char(ic);
                self.state = State::BogusDoctype;
            }
        }
    }

    fn step_after_doctype_system_keyword(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char(c, _) if is_html_whitespace(c) => {
                self.state = State::BeforeDoctypeSystemIdentifier
            }
            InputChar::Char('"', _) => {
                self.raise(
                    ParseErrorCode::MissingWhitespaceAfterDoctypeSystemKeyword,
                    pos,
                    Severity::Warning,
                );
                self.current_doctype.system_id = Some(String::new());
                self.state = State::DoctypeSystemIdentifierDoubleQuoted;
            }
            InputChar::Char('\'', _) => {
                self.raise(
                    ParseErrorCode::MissingWhitespaceAfterDoctypeSystemKeyword,
                    pos,
                    Severity::Warning,
                );
                self.current_doctype.system_id = Some(String::new());
                self.state = State::DoctypeSystemIdentifierSingleQuoted;
            }
            InputChar::Char('>', _) => {
                self.raise(
                    ParseErrorCode::MissingDoctypeSystemIdentifier,
                    pos,
                    Severity::Error,
                );
                self.current_doctype.force_quirks = true;
                self.emit_doctype();
                self.state = State::Data;
            }
            InputChar::Eof(_) => self.eof_in_doctype(pos),
            InputChar::Char(_, _) => {
                self.raise(
                    ParseErrorCode::MissingQuoteBeforeDoctypeSystemIdentifier,
                    pos,
                    Severity::Error,
                );
                self.current_doctype.force_quirks = true;
                self.unread_char(ic);
                self.state = State::BogusDoctype;
            }
        }
    }

    fn step_before_doctype_system_identifier(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char(c, _) if is_html_whitespace(c) => {}
            InputChar::Char('"', _) => {
                self.current_doctype.system_id = Some(String::new());
                self.state = State::DoctypeSystemIdentifierDoubleQuoted;
            }
            InputChar::Char('\'', _) => {
                self.current_doctype.system_id = Some(String::new());
                self.state = State::DoctypeSystemIdentifierSingleQuoted;
            }
            InputChar::Char('>', _) => {
                self.raise(
                    ParseErrorCode::MissingDoctypeSystemIdentifier,
                    pos,
                    Severity::Error,
                );
                self.current_doctype.force_quirks = true;
                self.emit_doctype();
                self.state = State::Data;
            }
            InputChar::Eof(_) => self.eof_in_doctype(pos),
            InputChar::Char(_, _) => {
                self.raise(
                    ParseErrorCode::MissingQuoteBeforeDoctypeSystemIdentifier,
                    pos,
                    Severity::Error,
                );
                self.current_doctype.force_quirks = true;
                self.unread_char(ic);
                self.state = State::BogusDoctype;
            }
        }
    }

    fn step_after_doctype_system_identifier(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char(c, _) if is_html_whitespace(c) => {}
            InputChar::Char('>', _) => {
                self.emit_doctype();
                self.state = State::Data;
            }
            InputChar::Eof(_) => self.eof_in_doctype(pos),
            InputChar::Char(_, _) => {
                self.raise(
                    ParseErrorCode::UnexpectedCharacterAfterDoctypeSystemIdentifier,
                    pos,
                    Severity::Warning,
                );
                self.unread_char(ic);
                self.state = State::BogusDoctype;
            }
        }
    }

    fn step_bogus_doctype(&mut self, ic: InputChar, _pos: u64) {
        if self.check_and_handle_span_limit(_pos) {
            self.unread_char(ic);
            return;
        }
        match ic {
            InputChar::Char('>', _) => {
                self.emit_doctype();
                self.state = State::Data;
            }
            InputChar::Eof(_) => {
                self.emit_doctype();
                self.finalize_eof();
            }
            InputChar::Char(_, _) => {}
        }
    }

    // --- CDATA (foreign content only; see limitations note in crate docs) ---

    fn step_cdata_section(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char(']', _) => self.state = State::CdataSectionBracket,
            InputChar::Char(c, _) => self.push_text_char(c, pos),
            InputChar::Eof(_) => {
                self.raise(ParseErrorCode::EofInCdata, pos, Severity::Error);
                self.finalize_eof();
            }
        }
    }

    fn step_cdata_section_bracket(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char(']', _) => self.state = State::CdataSectionEnd,
            _ => {
                self.push_text_char(']', pos);
                self.unread_char(ic);
                self.state = State::CdataSection;
            }
        }
    }

    fn step_cdata_section_end(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char(']', _) => self.push_text_char(']', pos),
            InputChar::Char('>', _) => self.state = State::Data,
            _ => {
                self.push_text_char(']', pos);
                self.push_text_char(']', pos);
                self.unread_char(ic);
                self.state = State::CdataSection;
            }
        }
    }

    // --- character references ---

    fn step_character_reference(&mut self, ic: InputChar) {
        match ic {
            InputChar::Char(c, _) if c.is_ascii_alphanumeric() => {
                self.scan_best_match = None;
                self.named_char_ref_scan_steps = 0;
                self.unread_char(ic);
                self.state = State::NamedCharacterReference;
            }
            InputChar::Char('#', _) => {
                self.char_ref_code = 0;
                self.char_ref_is_hex = false;
                self.state = State::NumericCharacterReference;
            }
            _ => {
                self.append_return_char('&');
                self.unread_char(ic);
                self.state = self.return_state;
            }
        }
    }

    fn step_named_character_reference(&mut self, ic: InputChar) {
        if let InputChar::Char(c, _) = ic {
            if c.is_ascii_alphanumeric() {
                self.named_char_ref_scan_steps += 1;
                if self.named_char_ref_scan_steps > self.limits.max_named_char_ref_scan_steps {
                    self.raise(
                        ParseErrorCode::LimitExceeded(LimitKind::NamedCharRefScanSteps),
                        ic.pos(),
                        Severity::Warning,
                    );
                    self.finalize_named_character_reference(ic);
                    return;
                }
                let mut tentative = self.scan_trail_as_string();
                tentative.push(c);
                if entities::has_prefix(ENTITY_TABLE, &tentative) {
                    self.scan_trail.push(ic);
                    if let Some((codepoint, legacy)) =
                        entities::exact_match(ENTITY_TABLE, &tentative)
                    {
                        self.scan_best_match = Some((self.scan_trail.len(), codepoint, legacy));
                    }
                    return;
                }
                self.finalize_named_character_reference(ic);
                return;
            }
        }
        self.finalize_named_character_reference(ic);
    }

    fn finalize_named_character_reference(&mut self, delimiter: InputChar) {
        let entity_name_len = self.scan_trail.len();
        let had_semicolon = matches!(delimiter, InputChar::Char(';', _));
        match self.scan_best_match.take() {
            Some((match_len, codepoint, legacy)) => {
                let immediately_followed_by_semicolon =
                    match_len == entity_name_len && had_semicolon;
                if immediately_followed_by_semicolon {
                    self.emit_char_ref_codepoint(codepoint);
                } else if legacy {
                    self.emit_char_ref_codepoint(codepoint);
                    let trail = std::mem::take(&mut self.scan_trail);
                    let mut remainder: Vec<InputChar> = trail.into_iter().skip(match_len).collect();
                    remainder.push(delimiter);
                    self.unread_sequence(remainder.into_iter());
                } else {
                    self.raise(
                        ParseErrorCode::MissingSemicolonAfterCharacterReference,
                        delimiter.pos(),
                        Severity::Warning,
                    );
                    self.emit_char_ref_codepoint(codepoint);
                    let trail = std::mem::take(&mut self.scan_trail);
                    let mut remainder: Vec<InputChar> = trail.into_iter().skip(match_len).collect();
                    remainder.push(delimiter);
                    self.unread_sequence(remainder.into_iter());
                }
            }
            None => {
                if entity_name_len > 0 && had_semicolon {
                    self.raise(
                        ParseErrorCode::UnknownNamedCharacterReference,
                        delimiter.pos(),
                        Severity::Warning,
                    );
                }
                self.flush_scan_trail_as_literal();
                self.unread_char(delimiter);
            }
        }
        self.scan_trail.clear();
        self.named_char_ref_scan_steps = 0;
        self.state = self.return_state;
    }

    fn step_numeric_character_reference(&mut self, ic: InputChar) {
        match ic {
            InputChar::Char(c @ ('x' | 'X'), _) => {
                self.char_ref_is_hex = true;
                self.char_ref_marker = c;
                self.state = State::HexadecimalCharacterReferenceStart;
            }
            _ => {
                self.char_ref_is_hex = false;
                self.unread_char(ic);
                self.state = State::DecimalCharacterReferenceStart;
            }
        }
    }

    fn step_hex_reference_start(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char(c, _) if c.is_ascii_hexdigit() => {
                self.unread_char(ic);
                self.state = State::HexadecimalCharacterReference;
            }
            _ => {
                self.raise(
                    ParseErrorCode::AbsenceOfDigitsInNumericCharacterReference,
                    pos,
                    Severity::Error,
                );
                self.flush_numeric_prefix_as_literal();
                self.unread_char(ic);
                self.state = self.return_state;
            }
        }
    }

    fn step_decimal_reference_start(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char(c, _) if c.is_ascii_digit() => {
                self.unread_char(ic);
                self.state = State::DecimalCharacterReference;
            }
            _ => {
                self.raise(
                    ParseErrorCode::AbsenceOfDigitsInNumericCharacterReference,
                    pos,
                    Severity::Error,
                );
                self.flush_numeric_prefix_as_literal();
                self.unread_char(ic);
                self.state = self.return_state;
            }
        }
    }

    fn step_hex_reference(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char(c, _) if c.is_ascii_hexdigit() => {
                let digit = c.to_digit(16).unwrap_or(0);
                self.char_ref_code = self.char_ref_code.saturating_mul(16).saturating_add(digit);
            }
            InputChar::Char(';', _) => self.state = State::NumericCharacterReferenceEnd,
            _ => {
                self.raise(
                    ParseErrorCode::MissingSemicolonAfterCharacterReference,
                    pos,
                    Severity::Warning,
                );
                self.unread_char(ic);
                self.state = State::NumericCharacterReferenceEnd;
            }
        }
    }

    fn step_decimal_reference(&mut self, ic: InputChar, pos: u64) {
        match ic {
            InputChar::Char(c, _) if c.is_ascii_digit() => {
                let digit = c.to_digit(10).unwrap_or(0);
                self.char_ref_code = self.char_ref_code.saturating_mul(10).saturating_add(digit);
            }
            InputChar::Char(';', _) => self.state = State::NumericCharacterReferenceEnd,
            _ => {
                self.raise(
                    ParseErrorCode::MissingSemicolonAfterCharacterReference,
                    pos,
                    Severity::Warning,
                );
                self.unread_char(ic);
                self.state = State::NumericCharacterReferenceEnd;
            }
        }
    }

    fn step_numeric_reference_end(&mut self, ic: InputChar) {
        let pos = ic.pos();
        self.unread_char(ic);
        let code = self.char_ref_code;
        let (final_char, diag) = numeric_ref_final(code);
        if let Some((code_kind, severity)) = diag {
            self.raise(code_kind, pos, severity);
        }
        self.append_return_char(final_char);
        self.char_ref_code = 0;
        self.state = self.return_state;
    }
}

fn numeric_ref_final(code: u32) -> (char, Option<(ParseErrorCode, Severity)>) {
    if code == 0 {
        return (
            '\u{FFFD}',
            Some((ParseErrorCode::NullCharacterReference, Severity::Error)),
        );
    }
    if code > 0x10FFFF {
        return (
            '\u{FFFD}',
            Some((
                ParseErrorCode::CharacterReferenceOutsideUnicodeRange,
                Severity::Error,
            )),
        );
    }
    if (0xD800..=0xDFFF).contains(&code) {
        return (
            '\u{FFFD}',
            Some((ParseErrorCode::SurrogateCharacterReference, Severity::Error)),
        );
    }
    if let Some(replacement) = entities::c1_control_replacement(code) {
        let ch = char::from_u32(replacement).unwrap_or('\u{FFFD}');
        return (ch, None);
    }
    let is_noncharacter = (0xFDD0..=0xFDEF).contains(&code) || (code & 0xFFFE) == 0xFFFE;
    if is_noncharacter {
        let ch = char::from_u32(code).unwrap_or('\u{FFFD}');
        return (
            ch,
            Some((
                ParseErrorCode::NoncharacterCharacterReference,
                Severity::Warning,
            )),
        );
    }
    let is_control = (code < 0x20 && code != 0x09 && code != 0x0A && code != 0x0C)
        || (0x7F..=0x9F).contains(&code);
    let ch = char::from_u32(code).unwrap_or('\u{FFFD}');
    if is_control {
        return (
            ch,
            Some((ParseErrorCode::ControlCharacterReference, Severity::Warning)),
        );
    }
    (ch, None)
}
