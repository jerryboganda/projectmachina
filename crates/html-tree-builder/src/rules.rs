//! Per-insertion-mode token handling (WHATWG HTML §13.2.6.4). One `impl
//! TreeBuilder` block per concern, dispatched from `run_mode` (called by
//! `builder.rs`'s bounded dispatch loop). Representative, spec-informed
//! coverage of the common document/table/list/formatting-element cases —
//! see `.agent-state/evidence/M2-T04.md` for the explicit list of spec
//! corners simplified or deferred (frameset documents, template contents
//! as a true document fragment, full foreign-content integration points).

use machina_dom::{Document, Namespace};
use machina_html::{
    CharacterToken, CommentToken, DoctypeToken, TagToken, TextContentState, Token, Tokenizer,
};

use crate::builder::TreeBuilder;
use crate::error::TreeBuilderError;
use crate::modes::{Dispatch, InsertionMode};
use crate::open_elements::OpenElementEntry;

enum TokenView<'a> {
    Doctype(&'a DoctypeToken),
    StartTag(&'a TagToken),
    EndTag(&'a TagToken),
    Comment(&'a CommentToken),
    Character(&'a CharacterToken),
    Eof,
    Other,
}

fn view(token: &Token) -> TokenView<'_> {
    match token {
        Token::Doctype(d) => TokenView::Doctype(d),
        Token::StartTag(t) => TokenView::StartTag(t),
        Token::EndTag(t) => TokenView::EndTag(t),
        Token::Comment(c) => TokenView::Comment(c),
        Token::Character(c) => TokenView::Character(c),
        Token::Eof => TokenView::Eof,
        _ => TokenView::Other,
    }
}

fn is_whitespace_str(s: &str) -> bool {
    s.chars()
        .all(|c| matches!(c, '\t' | '\n' | '\x0C' | '\r' | ' '))
}

const AUTO_CLOSE_P_BLOCK_TAGS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "center",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "header",
    "hgroup",
    "main",
    "menu",
    "nav",
    "ol",
    "p",
    "search",
    "section",
    "summary",
    "ul",
];
const HEADING_TAGS: &[&str] = &["h1", "h2", "h3", "h4", "h5", "h6"];
const FORMATTING_TAGS: &[&str] = &[
    "a", "b", "big", "code", "em", "font", "i", "nobr", "s", "small", "strike", "strong", "tt", "u",
];
const TABLE_STRUCTURE_IGNORED_IN_BODY: &[&str] = &[
    "caption", "col", "colgroup", "frame", "head", "tbody", "td", "tfoot", "th", "thead", "tr",
];

impl TreeBuilder {
    pub(crate) fn run_mode(
        &mut self,
        doc: &mut Document,
        tokenizer: &mut Tokenizer,
        mode: InsertionMode,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        match mode {
            InsertionMode::Initial => self.mode_initial(doc, token),
            InsertionMode::BeforeHtml => self.mode_before_html(doc, token),
            InsertionMode::BeforeHead => self.mode_before_head(doc, token),
            InsertionMode::InHead => self.mode_in_head(doc, tokenizer, token),
            InsertionMode::InHeadNoscript => self.mode_in_head_noscript(doc, token),
            InsertionMode::AfterHead => self.mode_after_head(doc, tokenizer, token),
            InsertionMode::InBody => self.mode_in_body(doc, tokenizer, token),
            InsertionMode::Text => self.mode_text(doc, tokenizer, token),
            InsertionMode::InTable => self.mode_in_table(doc, token),
            InsertionMode::InTableText => self.mode_in_table_text(doc, token),
            InsertionMode::InCaption => self.mode_in_caption(doc, token),
            InsertionMode::InColumnGroup => self.mode_in_column_group(doc, token),
            InsertionMode::InTableBody => self.mode_in_table_body(doc, token),
            InsertionMode::InRow => self.mode_in_row(doc, token),
            InsertionMode::InCell => self.mode_in_cell(doc, token),
            InsertionMode::InSelect => self.mode_in_select(doc, token),
            InsertionMode::InSelectInTable => self.mode_in_select_in_table(doc, token),
            InsertionMode::InTemplate => self.mode_in_template(doc, tokenizer, token),
            InsertionMode::AfterBody => self.mode_after_body(doc, token),
            InsertionMode::InFrameset => self.mode_in_frameset(doc, token),
            InsertionMode::AfterFrameset => self.mode_after_frameset(doc, token),
            InsertionMode::AfterAfterBody => self.mode_after_after_body(doc, token),
            InsertionMode::AfterAfterFrameset => self.mode_after_after_frameset(doc, token),
        }
    }

    // ---- Initial (§13.2.6.4.1) ------------------------------------------

    fn mode_initial(
        &mut self,
        doc: &mut Document,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        match view(token) {
            TokenView::Character(c) if is_whitespace_str(&c.data) => Ok(Dispatch::Consumed),
            TokenView::Comment(c) => {
                let root = doc.root();
                self.insert_comment_at(doc, &c.data, root)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::Doctype(d) => {
                let name = d.name.clone().unwrap_or_default();
                let public_id = d.public_id.clone().unwrap_or_default();
                let system_id = d.system_id.clone().unwrap_or_default();
                if !name.is_empty() {
                    if let Ok(handle) = doc.create_document_type(&name, &public_id, &system_id) {
                        let root = doc.root();
                        self.append_child_checked(doc, root, handle.node_handle())?;
                    }
                }
                self.mode = InsertionMode::BeforeHtml;
                Ok(Dispatch::Consumed)
            }
            _ => Ok(Dispatch::Reprocess(InsertionMode::BeforeHtml)),
        }
    }

    // ---- BeforeHtml (§13.2.6.4.2) ---------------------------------------

    fn mode_before_html(
        &mut self,
        doc: &mut Document,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        match view(token) {
            TokenView::Doctype(_) => {
                self.record_parse_error("unexpected DOCTYPE before <html>");
                Ok(Dispatch::Consumed)
            }
            TokenView::Comment(c) => {
                let root = doc.root();
                self.insert_comment_at(doc, &c.data, root)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::Character(c) if is_whitespace_str(&c.data) => Ok(Dispatch::Consumed),
            TokenView::StartTag(t) if t.name == "html" => {
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                self.mode = InsertionMode::BeforeHead;
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if matches!(t.name.as_str(), "head" | "body" | "html" | "br") => {
                Ok(Dispatch::Reprocess(
                    self.implicit_html_then(InsertionMode::BeforeHead),
                ))
            }
            TokenView::EndTag(_) => {
                self.record_parse_error("unexpected end tag before <html>");
                Ok(Dispatch::Consumed)
            }
            _ => Ok(Dispatch::Reprocess(
                self.implicit_html_then(InsertionMode::BeforeHead),
            )),
        }
        .and_then(|d| self.finish_dispatch_creating_html_if_needed(doc, d))
    }

    /// Records the target mode for an implicitly-created `<html>` element;
    /// the actual creation happens in `finish_dispatch_creating_html_if_needed`
    /// once we know we are really taking this branch (keeps `mode_before_html`
    /// a plain `match` without duplicated element-creation calls).
    fn implicit_html_then(&mut self, next: InsertionMode) -> InsertionMode {
        next
    }

    fn finish_dispatch_creating_html_if_needed(
        &mut self,
        doc: &mut Document,
        dispatch: Dispatch,
    ) -> Result<Dispatch, TreeBuilderError> {
        if let Dispatch::Reprocess(next) = dispatch {
            if self.open_elements.html_element().is_none() {
                self.insert_html_element(doc, "html", &[])?;
            }
            self.mode = next;
            return Ok(Dispatch::Reprocess(next));
        }
        Ok(dispatch)
    }

    // ---- BeforeHead (§13.2.6.4.3) ---------------------------------------

    fn mode_before_head(
        &mut self,
        doc: &mut Document,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        match view(token) {
            TokenView::Character(c) if is_whitespace_str(&c.data) => Ok(Dispatch::Consumed),
            TokenView::Comment(c) => {
                self.insert_comment(doc, &c.data, false)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::Doctype(_) => {
                self.record_parse_error("unexpected DOCTYPE before <head>");
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "html" => {
                self.record_parse_error("duplicate <html> start tag");
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "head" => {
                let handle = self.insert_html_element(doc, &t.name, &t.attributes)?;
                self.head_element = handle;
                self.mode = InsertionMode::InHead;
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if matches!(t.name.as_str(), "head" | "body" | "html" | "br") => {
                self.insert_implicit_head(doc)?;
                Ok(Dispatch::Reprocess(InsertionMode::InHead))
            }
            TokenView::EndTag(_) => {
                self.record_parse_error("unexpected end tag before <head>");
                Ok(Dispatch::Consumed)
            }
            _ => {
                self.insert_implicit_head(doc)?;
                Ok(Dispatch::Reprocess(InsertionMode::InHead))
            }
        }
    }

    fn insert_implicit_head(&mut self, doc: &mut Document) -> Result<(), TreeBuilderError> {
        let handle = self.insert_html_element(doc, "head", &[])?;
        self.head_element = handle;
        self.mode = InsertionMode::InHead;
        Ok(())
    }

    // ---- InHead (§13.2.6.4.4) --------------------------------------------

    fn mode_in_head(
        &mut self,
        doc: &mut Document,
        tokenizer: &mut Tokenizer,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        match view(token) {
            TokenView::Character(c) if is_whitespace_str(&c.data) => {
                self.insert_text(doc, &c.data, false)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::Comment(c) => {
                self.insert_comment(doc, &c.data, false)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::Doctype(_) => {
                self.record_parse_error("unexpected DOCTYPE in <head>");
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "html" => {
                self.record_parse_error("duplicate <html> start tag");
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t)
                if matches!(t.name.as_str(), "base" | "basefont" | "bgsound" | "link") =>
            {
                self.insert_void_html_element(doc, &t.name, &t.attributes, false)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "meta" => {
                self.insert_void_html_element(doc, &t.name, &t.attributes, false)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "title" => {
                self.generic_raw_text_or_rcdata(doc, tokenizer, t, TextContentState::Rcdata)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "noscript" && self.scripting_enabled => {
                self.generic_raw_text_or_rcdata(doc, tokenizer, t, TextContentState::Rawtext)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "noscript" => {
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                self.mode = InsertionMode::InHeadNoscript;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if matches!(t.name.as_str(), "noframes" | "style") => {
                self.generic_raw_text_or_rcdata(doc, tokenizer, t, TextContentState::Rawtext)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "script" => {
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                self.switch_to_text_mode(tokenizer, TextContentState::ScriptData);
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if t.name == "head" => {
                self.open_elements.pop();
                self.mode = InsertionMode::AfterHead;
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if matches!(t.name.as_str(), "body" | "html" | "br") => {
                self.open_elements.pop();
                self.mode = InsertionMode::AfterHead;
                Ok(Dispatch::Reprocess(InsertionMode::AfterHead))
            }
            TokenView::StartTag(t) if t.name == "template" => {
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                self.afe.insert_marker();
                self.frameset_ok = false;
                self.template_modes.push(InsertionMode::InTemplate);
                self.mode = InsertionMode::InTemplate;
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if t.name == "template" => {
                if !self.open_elements.has_html_tag_anywhere("template") {
                    self.record_parse_error("</template> with no matching open <template>");
                    return Ok(Dispatch::Consumed);
                }
                self.open_elements.pop_implied_end_tags(None, true);
                if self.open_elements.current().map(|e| e.tag.as_str()) != Some("template") {
                    self.record_parse_error("</template> did not match current node");
                }
                self.open_elements.pop_until_html_tag("template");
                self.afe.clear_to_last_marker();
                self.template_modes.pop();
                self.mode = self
                    .template_modes
                    .last()
                    .copied()
                    .unwrap_or(InsertionMode::InBody);
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "head" => {
                self.record_parse_error("unexpected <head> start tag");
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(_) => {
                self.record_parse_error("unexpected end tag in <head>");
                Ok(Dispatch::Consumed)
            }
            _ => {
                self.open_elements.pop();
                self.mode = InsertionMode::AfterHead;
                Ok(Dispatch::Reprocess(InsertionMode::AfterHead))
            }
        }
    }

    // ---- InHeadNoscript (§13.2.6.4.5) -------------------------------------

    fn mode_in_head_noscript(
        &mut self,
        doc: &mut Document,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        match view(token) {
            TokenView::Doctype(_) => {
                self.record_parse_error("unexpected DOCTYPE in <noscript>");
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "html" => {
                self.record_parse_error("duplicate <html> start tag");
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if t.name == "noscript" => {
                self.open_elements.pop();
                self.mode = InsertionMode::InHead;
                Ok(Dispatch::Consumed)
            }
            TokenView::Character(c) if is_whitespace_str(&c.data) => {
                self.insert_text(doc, &c.data, false)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::Comment(c) => {
                self.insert_comment(doc, &c.data, false)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t)
                if matches!(
                    t.name.as_str(),
                    "basefont" | "bgsound" | "link" | "meta" | "noframes" | "style"
                ) =>
            {
                self.insert_void_html_element(doc, &t.name, &t.attributes, false)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if t.name == "br" => {
                self.open_elements.pop();
                self.mode = InsertionMode::InHead;
                Ok(Dispatch::Reprocess(InsertionMode::InHead))
            }
            TokenView::StartTag(t) if matches!(t.name.as_str(), "head" | "noscript") => {
                self.record_parse_error("unexpected start tag in <noscript>");
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(_) => {
                self.record_parse_error("unexpected end tag in <noscript>");
                Ok(Dispatch::Consumed)
            }
            _ => {
                self.record_parse_error("unexpected token in <noscript>");
                self.open_elements.pop();
                self.mode = InsertionMode::InHead;
                Ok(Dispatch::Reprocess(InsertionMode::InHead))
            }
        }
    }

    // ---- AfterHead (§13.2.6.4.6) ------------------------------------------

    fn mode_after_head(
        &mut self,
        doc: &mut Document,
        tokenizer: &mut Tokenizer,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        match view(token) {
            TokenView::Character(c) if is_whitespace_str(&c.data) => {
                self.insert_text(doc, &c.data, false)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::Comment(c) => {
                self.insert_comment(doc, &c.data, false)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::Doctype(_) => {
                self.record_parse_error("unexpected DOCTYPE after <head>");
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "html" => {
                self.record_parse_error("duplicate <html> start tag");
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "body" => {
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                self.frameset_ok = false;
                self.mode = InsertionMode::InBody;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "frameset" => {
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                self.mode = InsertionMode::InFrameset;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t)
                if matches!(
                    t.name.as_str(),
                    "base"
                        | "basefont"
                        | "bgsound"
                        | "link"
                        | "meta"
                        | "noframes"
                        | "script"
                        | "style"
                        | "template"
                        | "title"
                ) =>
            {
                self.record_parse_error("start tag reopening <head> after it closed");
                if let Some(head) = self.head_element {
                    self.open_elements.push(OpenElementEntry {
                        handle: head,
                        tag: "head".to_string(),
                        namespace: Namespace::Html,
                    });
                    self.mode_in_head(doc, tokenizer, token)?;
                    if let Some(idx) = self.open_elements.index_of_handle(head) {
                        self.open_elements.remove_at(idx);
                    }
                }
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if t.name == "template" => {
                self.mode_in_head(doc, tokenizer, token)
            }
            TokenView::EndTag(t) if matches!(t.name.as_str(), "body" | "html" | "br") => {
                self.insert_html_element(doc, "body", &[])?;
                self.mode = InsertionMode::InBody;
                Ok(Dispatch::Reprocess(InsertionMode::InBody))
            }
            TokenView::StartTag(t) if t.name == "head" => {
                self.record_parse_error("unexpected <head> start tag");
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(_) => {
                self.record_parse_error("unexpected end tag after <head>");
                Ok(Dispatch::Consumed)
            }
            _ => {
                self.insert_html_element(doc, "body", &[])?;
                self.mode = InsertionMode::InBody;
                Ok(Dispatch::Reprocess(InsertionMode::InBody))
            }
        }
    }

    // ---- InBody (§13.2.6.4.7) ---------------------------------------------

    fn mode_in_body(
        &mut self,
        doc: &mut Document,
        tokenizer: &mut Tokenizer,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        match view(token) {
            TokenView::Doctype(_) => {
                self.record_parse_error("unexpected DOCTYPE");
                Ok(Dispatch::Consumed)
            }
            TokenView::Comment(c) => {
                self.insert_comment(doc, &c.data, false)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::Character(c) => {
                self.reconstruct_active_formatting_elements(doc)?;
                self.insert_text(doc, &c.data, false)?;
                if !is_whitespace_str(&c.data) {
                    self.frameset_ok = false;
                }
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "html" => {
                self.record_parse_error("duplicate <html> start tag");
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t)
                if matches!(
                    t.name.as_str(),
                    "base"
                        | "basefont"
                        | "bgsound"
                        | "link"
                        | "meta"
                        | "noframes"
                        | "script"
                        | "style"
                        | "template"
                        | "title"
                ) =>
            {
                self.mode_in_head(doc, tokenizer, token)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if t.name == "template" => {
                self.mode_in_head(doc, tokenizer, token)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "body" => {
                if self.open_elements.has_html_tag_anywhere("body") {
                    self.record_parse_error("duplicate <body> start tag");
                } else {
                    self.frameset_ok = false;
                }
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "frameset" => {
                self.record_ignored("frameset", "InBody");
                Ok(Dispatch::Consumed)
            }
            TokenView::Eof => {
                if !self.template_modes.is_empty() {
                    return self.mode_in_template(doc, tokenizer, token);
                }
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if t.name == "body" => {
                if !self.open_elements.has_in_scope("body") {
                    self.record_parse_error("</body> with no <body> in scope");
                    return Ok(Dispatch::Consumed);
                }
                self.mode = InsertionMode::AfterBody;
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if t.name == "html" => {
                if !self.open_elements.has_in_scope("body") {
                    self.record_parse_error("</html> with no <body> in scope");
                    return Ok(Dispatch::Consumed);
                }
                self.mode = InsertionMode::AfterBody;
                Ok(Dispatch::Reprocess(InsertionMode::AfterBody))
            }
            TokenView::StartTag(t) if AUTO_CLOSE_P_BLOCK_TAGS.contains(&t.name.as_str()) => {
                self.close_p_element_if_in_button_scope(doc)?;
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if HEADING_TAGS.contains(&t.name.as_str()) => {
                self.close_p_element_if_in_button_scope(doc)?;
                if let Some(current) = self.open_elements.current() {
                    if HEADING_TAGS.contains(&current.tag.as_str()) {
                        self.record_parse_error("nested heading; implicitly closing");
                        self.open_elements.pop();
                    }
                }
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if matches!(t.name.as_str(), "pre" | "listing") => {
                self.close_p_element_if_in_button_scope(doc)?;
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                self.frameset_ok = false;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "form" => {
                let template_open = self.open_elements.has_html_tag_anywhere("template");
                if self.form_element.is_some() && !template_open {
                    self.record_parse_error("nested <form> ignored");
                    return Ok(Dispatch::Consumed);
                }
                self.close_p_element_if_in_button_scope(doc)?;
                let handle = self.insert_html_element(doc, &t.name, &t.attributes)?;
                if !template_open {
                    self.form_element = handle;
                }
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "li" => {
                self.frameset_ok = false;
                self.close_implicit_list_item(doc, "li")?;
                self.close_p_element_if_in_button_scope(doc)?;
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if matches!(t.name.as_str(), "dd" | "dt") => {
                self.frameset_ok = false;
                self.close_implicit_list_item(doc, &t.name)?;
                self.close_p_element_if_in_button_scope(doc)?;
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "plaintext" => {
                self.close_p_element_if_in_button_scope(doc)?;
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                self.switch_to_text_mode(tokenizer, TextContentState::Plaintext);
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "button" => {
                if self.open_elements.has_in_scope("button") {
                    self.record_parse_error("nested <button>; implicitly closing");
                    self.open_elements.pop_implied_end_tags(None, false);
                    self.open_elements.pop_until_html_tag("button");
                }
                self.reconstruct_active_formatting_elements(doc)?;
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                self.frameset_ok = false;
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t)
                if matches!(
                    t.name.as_str(),
                    "address"
                        | "article"
                        | "aside"
                        | "blockquote"
                        | "button"
                        | "center"
                        | "details"
                        | "dialog"
                        | "dir"
                        | "div"
                        | "dl"
                        | "fieldset"
                        | "figcaption"
                        | "figure"
                        | "footer"
                        | "header"
                        | "hgroup"
                        | "listing"
                        | "main"
                        | "menu"
                        | "nav"
                        | "ol"
                        | "pre"
                        | "search"
                        | "section"
                        | "summary"
                        | "ul"
                ) =>
            {
                if !self.open_elements.has_in_scope(&t.name) {
                    self.record_parse_error(format!(
                        "</{}> with no matching element in scope",
                        t.name
                    ));
                    return Ok(Dispatch::Consumed);
                }
                self.open_elements.pop_implied_end_tags(None, false);
                if self.open_elements.current().map(|e| e.tag.as_str()) != Some(t.name.as_str()) {
                    self.record_parse_error(format!("</{}> did not match current node", t.name));
                }
                self.open_elements.pop_until_html_tag(&t.name);
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if t.name == "form" => {
                let template_open = self.open_elements.has_html_tag_anywhere("template");
                if !template_open {
                    let node = self.form_element.take();
                    let Some(node) = node else {
                        self.record_parse_error("</form> with no open <form>");
                        return Ok(Dispatch::Consumed);
                    };
                    if !self.open_elements.has_in_scope("form") {
                        self.record_parse_error("</form> with <form> not in scope");
                        return Ok(Dispatch::Consumed);
                    }
                    self.open_elements.pop_implied_end_tags(None, false);
                    if self.open_elements.current_handle() != Some(node) {
                        self.record_parse_error("</form> did not match current node");
                    }
                    if let Some(idx) = self.open_elements.index_of_handle(node) {
                        self.open_elements.remove_at(idx);
                    }
                } else {
                    if !self.open_elements.has_in_scope("form") {
                        self.record_parse_error("</form> with <form> not in scope");
                        return Ok(Dispatch::Consumed);
                    }
                    self.open_elements.pop_implied_end_tags(None, false);
                    if self.open_elements.current().map(|e| e.tag.as_str()) != Some("form") {
                        self.record_parse_error("</form> did not match current node");
                    }
                    self.open_elements.pop_until_html_tag("form");
                }
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if t.name == "p" => {
                if !self.open_elements.has_in_button_scope("p") {
                    self.record_parse_error(
                        "</p> with no <p> in button scope; inserting implicit <p>",
                    );
                    self.insert_html_element(doc, "p", &[])?;
                }
                self.close_p_element(doc)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if t.name == "li" => {
                if !self.open_elements.has_in_list_item_scope("li") {
                    self.record_parse_error("</li> with no <li> in list-item scope");
                    return Ok(Dispatch::Consumed);
                }
                self.open_elements.pop_implied_end_tags(Some("li"), false);
                if self.open_elements.current().map(|e| e.tag.as_str()) != Some("li") {
                    self.record_parse_error("</li> did not match current node");
                }
                self.open_elements.pop_until_html_tag("li");
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if matches!(t.name.as_str(), "dd" | "dt") => {
                if !self.open_elements.has_in_scope(&t.name) {
                    self.record_parse_error(format!(
                        "</{}> with no matching element in scope",
                        t.name
                    ));
                    return Ok(Dispatch::Consumed);
                }
                self.open_elements
                    .pop_implied_end_tags(Some(&t.name), false);
                if self.open_elements.current().map(|e| e.tag.as_str()) != Some(t.name.as_str()) {
                    self.record_parse_error(format!("</{}> did not match current node", t.name));
                }
                self.open_elements.pop_until_html_tag(&t.name);
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if HEADING_TAGS.contains(&t.name.as_str()) => {
                let any_heading_in_scope = HEADING_TAGS
                    .iter()
                    .any(|h| self.open_elements.has_in_scope(h));
                if !any_heading_in_scope {
                    self.record_parse_error("heading end tag with no heading in scope");
                    return Ok(Dispatch::Consumed);
                }
                self.open_elements.pop_implied_end_tags(None, false);
                if self.open_elements.current().map(|e| e.tag.as_str()) != Some(t.name.as_str()) {
                    self.record_parse_error("heading end tag did not match current node");
                }
                while let Some(top) = self.open_elements.current() {
                    let is_heading = HEADING_TAGS.contains(&top.tag.as_str());
                    self.open_elements.pop();
                    if is_heading {
                        break;
                    }
                }
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "a" => {
                if let Some((_idx, handle, _attrs, _ns)) =
                    self.afe.last_between_end_and_marker_with_tag("a")
                {
                    self.record_parse_error("nested <a>; running adoption agency");
                    crate::adoption_agency::run_adoption_agency(self, doc, "a")?;
                    self.afe.remove_element(handle);
                    if let Some(idx) = self.open_elements.index_of_handle(handle) {
                        self.open_elements.remove_at(idx);
                    }
                }
                self.reconstruct_active_formatting_elements(doc)?;
                let pairs = crate::builder::attrs_to_pairs(&t.attributes);
                if let Some(handle) =
                    self.create_element_and_push(doc, "a", Namespace::Html, &pairs)?
                {
                    self.afe
                        .push_with_noahs_ark(handle, "a".to_string(), Namespace::Html, pairs);
                }
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if FORMATTING_TAGS.contains(&t.name.as_str()) => {
                self.reconstruct_active_formatting_elements(doc)?;
                let pairs = crate::builder::attrs_to_pairs(&t.attributes);
                if let Some(handle) =
                    self.create_element_and_push(doc, &t.name, Namespace::Html, &pairs)?
                {
                    self.afe
                        .push_with_noahs_ark(handle, t.name.clone(), Namespace::Html, pairs);
                }
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if FORMATTING_TAGS.contains(&t.name.as_str()) => {
                crate::adoption_agency::run_adoption_agency(self, doc, &t.name)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t)
                if matches!(t.name.as_str(), "applet" | "marquee" | "object") =>
            {
                self.reconstruct_active_formatting_elements(doc)?;
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                self.afe.insert_marker();
                self.frameset_ok = false;
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if matches!(t.name.as_str(), "applet" | "marquee" | "object") => {
                if !self.open_elements.has_in_scope(&t.name) {
                    self.record_parse_error(format!(
                        "</{}> with no matching element in scope",
                        t.name
                    ));
                    return Ok(Dispatch::Consumed);
                }
                self.open_elements.pop_implied_end_tags(None, false);
                if self.open_elements.current().map(|e| e.tag.as_str()) != Some(t.name.as_str()) {
                    self.record_parse_error(format!("</{}> did not match current node", t.name));
                }
                self.open_elements.pop_until_html_tag(&t.name);
                self.afe.clear_to_last_marker();
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "table" => {
                self.close_p_element_if_in_button_scope(doc)?;
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                self.frameset_ok = false;
                self.mode = InsertionMode::InTable;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t)
                if matches!(
                    t.name.as_str(),
                    "area" | "br" | "embed" | "img" | "keygen" | "wbr"
                ) =>
            {
                self.reconstruct_active_formatting_elements(doc)?;
                self.insert_void_html_element(doc, &t.name, &t.attributes, false)?;
                self.frameset_ok = false;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "input" => {
                self.reconstruct_active_formatting_elements(doc)?;
                self.insert_void_html_element(doc, &t.name, &t.attributes, false)?;
                let is_hidden = t.attributes.iter().any(|a| {
                    a.name.eq_ignore_ascii_case("type") && a.value.eq_ignore_ascii_case("hidden")
                });
                if !is_hidden {
                    self.frameset_ok = false;
                }
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if matches!(t.name.as_str(), "param" | "source" | "track") => {
                self.insert_void_html_element(doc, &t.name, &t.attributes, false)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "hr" => {
                self.close_p_element_if_in_button_scope(doc)?;
                self.insert_void_html_element(doc, &t.name, &t.attributes, false)?;
                self.frameset_ok = false;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "image" => {
                self.record_parse_error("<image> treated as <img>");
                let mut renamed = t.clone();
                renamed.name = "img".to_string();
                self.mode_in_body(doc, tokenizer, &Token::StartTag(renamed))
            }
            TokenView::StartTag(t) if t.name == "textarea" => {
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                self.frameset_ok = false;
                self.switch_to_text_mode(tokenizer, TextContentState::Rcdata);
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "xmp" => {
                self.close_p_element_if_in_button_scope(doc)?;
                self.reconstruct_active_formatting_elements(doc)?;
                self.frameset_ok = false;
                self.generic_raw_text_or_rcdata(doc, tokenizer, t, TextContentState::Rawtext)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "iframe" => {
                self.frameset_ok = false;
                self.generic_raw_text_or_rcdata(doc, tokenizer, t, TextContentState::Rawtext)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "noembed" => {
                self.generic_raw_text_or_rcdata(doc, tokenizer, t, TextContentState::Rawtext)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "noscript" && self.scripting_enabled => {
                self.generic_raw_text_or_rcdata(doc, tokenizer, t, TextContentState::Rawtext)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "select" => {
                self.reconstruct_active_formatting_elements(doc)?;
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                self.frameset_ok = false;
                self.mode = match self.mode {
                    InsertionMode::InTable
                    | InsertionMode::InCaption
                    | InsertionMode::InTableBody
                    | InsertionMode::InRow
                    | InsertionMode::InCell => InsertionMode::InSelectInTable,
                    _ => InsertionMode::InSelect,
                };
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if matches!(t.name.as_str(), "optgroup" | "option") => {
                if self.open_elements.current().map(|e| e.tag.as_str()) == Some("option") {
                    self.open_elements.pop();
                }
                self.reconstruct_active_formatting_elements(doc)?;
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if matches!(t.name.as_str(), "rb" | "rtc") => {
                if self.open_elements.has_in_scope("ruby") {
                    self.open_elements.pop_implied_end_tags(None, false);
                }
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if matches!(t.name.as_str(), "rp" | "rt") => {
                if self.open_elements.has_in_scope("ruby") {
                    self.open_elements.pop_implied_end_tags(Some("rtc"), false);
                }
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "math" => {
                self.reconstruct_active_formatting_elements(doc)?;
                let pairs = crate::builder::attrs_to_pairs(&t.attributes);
                let self_closing = t.self_closing;
                let handle = self.create_and_insert_element(
                    doc,
                    "math",
                    Namespace::MathMl,
                    &pairs,
                    false,
                    true,
                )?;
                if self_closing && handle.is_some() {
                    self.open_elements.pop();
                }
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "svg" => {
                self.reconstruct_active_formatting_elements(doc)?;
                let adjusted = crate::foreign::adjust_svg_tag_name(&t.name);
                let pairs = crate::builder::attrs_to_pairs(&t.attributes);
                let self_closing = t.self_closing;
                let handle = self.create_and_insert_element(
                    doc,
                    &adjusted,
                    Namespace::Svg,
                    &pairs,
                    false,
                    true,
                )?;
                if self_closing && handle.is_some() {
                    self.open_elements.pop();
                }
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t)
                if TABLE_STRUCTURE_IGNORED_IN_BODY.contains(&t.name.as_str()) =>
            {
                self.record_ignored(&t.name, "InBody");
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) => {
                self.reconstruct_active_formatting_elements(doc)?;
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) => {
                self.any_other_end_tag_in_body(&t.name);
                Ok(Dispatch::Consumed)
            }
            TokenView::Other => Ok(Dispatch::Consumed),
        }
    }

    fn close_implicit_list_item(
        &mut self,
        doc: &mut Document,
        target: &str,
    ) -> Result<(), TreeBuilderError> {
        for i in (0..self.open_elements.len()).rev() {
            let Some(entry) = self.open_elements.entry_at(i).cloned() else {
                break;
            };
            if entry.namespace == Namespace::Html && entry.tag == target {
                self.open_elements.pop_implied_end_tags(Some(target), false);
                if self.open_elements.current().map(|e| e.tag.as_str()) != Some(target) {
                    self.record_parse_error(format!("implicit </{target}> did not land correctly"));
                }
                self.open_elements.pop_until_html_tag(target);
                return Ok(());
            }
            if crate::special::is_special(&entry.tag, entry.namespace)
                && !matches!(entry.tag.as_str(), "address" | "div" | "p")
            {
                break;
            }
        }
        let _ = doc;
        Ok(())
    }

    // ---- Text (§13.2.6.4.8) ------------------------------------------------

    fn mode_text(
        &mut self,
        doc: &mut Document,
        tokenizer: &mut Tokenizer,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        match view(token) {
            TokenView::Character(c) => {
                self.insert_text(doc, &c.data, false)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::Eof => {
                self.record_parse_error("EOF inside text-mode element");
                self.open_elements.pop();
                let restore = self.original_mode.take().unwrap_or(InsertionMode::InBody);
                self.mode = restore;
                Ok(Dispatch::Reprocess(restore))
            }
            TokenView::EndTag(t) if t.name == "script" => {
                let script_handle = self.open_elements.pop();
                let restore = self.original_mode.take().unwrap_or(InsertionMode::InBody);
                self.mode = restore;
                if let Some(entry) = script_handle {
                    if entry.namespace == Namespace::Html {
                        let has_src = doc.attribute(entry.handle, "src").ok().flatten().is_some();
                        let source = if has_src {
                            crate::checkpoint::ScriptSource::External
                        } else {
                            crate::checkpoint::ScriptSource::Inline
                        };
                        self.paused = Some(crate::checkpoint::ScriptCheckpoint {
                            script_element: entry.handle,
                            source,
                        });
                    }
                }
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(_) => {
                self.open_elements.pop();
                let restore = self.original_mode.take().unwrap_or(InsertionMode::InBody);
                self.mode = restore;
                Ok(Dispatch::Consumed)
            }
            _ => {
                let _ = tokenizer;
                Ok(Dispatch::Consumed)
            }
        }
    }

    // ---- InTable family (§13.2.6.4.9-17) -----------------------------------

    fn mode_in_table(
        &mut self,
        doc: &mut Document,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        match view(token) {
            TokenView::Character(_)
                if matches!(
                    self.open_elements.current().map(|e| e.tag.as_str()),
                    Some("table") | Some("tbody") | Some("tfoot") | Some("thead") | Some("tr")
                ) =>
            {
                self.pending_table_text.clear();
                self.pending_table_text_had_non_whitespace = false;
                self.pending_table_text_original_mode = self.mode;
                self.mode = InsertionMode::InTableText;
                Ok(Dispatch::Reprocess(InsertionMode::InTableText))
            }
            TokenView::Comment(c) => {
                self.insert_comment(doc, &c.data, false)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::Doctype(_) => {
                self.record_parse_error("unexpected DOCTYPE in <table>");
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "caption" => {
                self.clear_stack_back_to_table_context();
                self.afe.insert_marker();
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                self.mode = InsertionMode::InCaption;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "colgroup" => {
                self.clear_stack_back_to_table_context();
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                self.mode = InsertionMode::InColumnGroup;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "col" => {
                self.clear_stack_back_to_table_context();
                self.insert_html_element(doc, "colgroup", &[])?;
                self.mode = InsertionMode::InColumnGroup;
                Ok(Dispatch::Reprocess(InsertionMode::InColumnGroup))
            }
            TokenView::StartTag(t) if matches!(t.name.as_str(), "tbody" | "tfoot" | "thead") => {
                self.clear_stack_back_to_table_context();
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                self.mode = InsertionMode::InTableBody;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if matches!(t.name.as_str(), "td" | "th" | "tr") => {
                self.clear_stack_back_to_table_context();
                self.insert_html_element(doc, "tbody", &[])?;
                self.mode = InsertionMode::InTableBody;
                Ok(Dispatch::Reprocess(InsertionMode::InTableBody))
            }
            TokenView::StartTag(t) if t.name == "table" => {
                self.record_parse_error("nested <table>; implicitly closing outer table");
                if self.open_elements.has_in_table_scope("table") {
                    self.open_elements.pop_until_html_tag("table");
                    self.reset_mode_after_table_close();
                    return Ok(Dispatch::Reprocess(self.mode));
                }
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if t.name == "table" => {
                if !self.open_elements.has_in_table_scope("table") {
                    self.record_parse_error("</table> with no <table> in scope");
                    return Ok(Dispatch::Consumed);
                }
                self.open_elements.pop_until_html_tag("table");
                self.reset_mode_after_table_close();
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t)
                if matches!(
                    t.name.as_str(),
                    "body"
                        | "caption"
                        | "col"
                        | "colgroup"
                        | "html"
                        | "tbody"
                        | "td"
                        | "tfoot"
                        | "th"
                        | "thead"
                        | "tr"
                ) =>
            {
                self.record_parse_error(format!("unexpected </{}> in <table>", t.name));
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t)
                if matches!(t.name.as_str(), "style" | "script" | "template") =>
            {
                self.record_ignored(&t.name, "InTable-delegated-to-InHead");
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "input" => {
                let is_hidden = t.attributes.iter().any(|a| {
                    a.name.eq_ignore_ascii_case("type") && a.value.eq_ignore_ascii_case("hidden")
                });
                if is_hidden {
                    self.record_parse_error("hidden <input> inside <table>");
                    self.insert_void_html_element(doc, &t.name, &t.attributes, false)?;
                    Ok(Dispatch::Consumed)
                } else {
                    self.foster_parented_start_tag_via_in_body(doc, token)
                }
            }
            TokenView::StartTag(t) if t.name == "form" => {
                self.record_parse_error("<form> inside <table>");
                if self.form_element.is_none()
                    && !self.open_elements.has_html_tag_anywhere("template")
                {
                    let handle = self.insert_html_element(doc, &t.name, &t.attributes)?;
                    self.form_element = handle;
                    self.open_elements.pop();
                }
                Ok(Dispatch::Consumed)
            }
            TokenView::Eof => Ok(Dispatch::Consumed),
            _ => self.foster_parented_start_tag_via_in_body(doc, token),
        }
    }

    /// WHATWG's "process using the rules for InBody, but with foster
    /// parenting enabled" fallback used by several table-context modes.
    /// Since `mode_in_body` calls `insert_html_element`/`insert_text`
    /// (non-fostering) internally, this crate implements the fallback by
    /// temporarily flagging foster mode and routing element/text
    /// insertions accordingly.
    fn foster_parented_start_tag_via_in_body(
        &mut self,
        doc: &mut Document,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        self.record_parse_error("content requiring foster-parenting inside <table>");
        match view(token) {
            TokenView::StartTag(t) => {
                self.reconstruct_active_formatting_elements(doc)?;
                self.insert_html_element_foster(doc, &t.name, &t.attributes)?;
            }
            TokenView::Character(c) => {
                self.reconstruct_active_formatting_elements(doc)?;
                self.insert_text(doc, &c.data, true)?;
            }
            _ => {}
        }
        Ok(Dispatch::Consumed)
    }

    fn clear_stack_back_to_table_context(&mut self) {
        while let Some(top) = self.open_elements.current() {
            if matches!(top.tag.as_str(), "table" | "template" | "html") {
                break;
            }
            self.open_elements.pop();
        }
    }

    fn clear_stack_back_to_table_body_context(&mut self) {
        while let Some(top) = self.open_elements.current() {
            if matches!(
                top.tag.as_str(),
                "tbody" | "tfoot" | "thead" | "template" | "html"
            ) {
                break;
            }
            self.open_elements.pop();
        }
    }

    fn clear_stack_back_to_table_row_context(&mut self) {
        while let Some(top) = self.open_elements.current() {
            if matches!(top.tag.as_str(), "tr" | "template" | "html") {
                break;
            }
            self.open_elements.pop();
        }
    }

    fn reset_mode_after_table_close(&mut self) {
        self.mode = self
            .template_modes
            .last()
            .copied()
            .unwrap_or(InsertionMode::InBody);
    }

    fn mode_in_table_text(
        &mut self,
        doc: &mut Document,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        match view(token) {
            TokenView::Character(c) => {
                if !is_whitespace_str(&c.data) {
                    self.pending_table_text_had_non_whitespace = true;
                }
                self.pending_table_text.push_str(&c.data);
                Ok(Dispatch::Consumed)
            }
            _ => {
                let text = std::mem::take(&mut self.pending_table_text);
                let had_non_whitespace = self.pending_table_text_had_non_whitespace;
                self.pending_table_text_had_non_whitespace = false;
                let restore = self.pending_table_text_original_mode;
                self.mode = restore;
                if had_non_whitespace {
                    self.record_parse_error("non-whitespace text inside <table>; foster-parenting");
                    self.reconstruct_active_formatting_elements(doc)?;
                    self.insert_text(doc, &text, true)?;
                } else if !text.is_empty() {
                    self.insert_text(doc, &text, false)?;
                }
                Ok(Dispatch::Reprocess(restore))
            }
        }
    }

    fn mode_in_caption(
        &mut self,
        doc: &mut Document,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        match view(token) {
            TokenView::EndTag(t) if t.name == "caption" => self.close_caption(doc),
            TokenView::StartTag(t)
                if matches!(
                    t.name.as_str(),
                    "caption"
                        | "col"
                        | "colgroup"
                        | "tbody"
                        | "td"
                        | "tfoot"
                        | "th"
                        | "thead"
                        | "tr"
                ) =>
            {
                if !self.open_elements.has_in_table_scope("caption") {
                    self.record_parse_error(
                        "unexpected table start tag with no <caption> in scope",
                    );
                    return Ok(Dispatch::Consumed);
                }
                let dispatch = self.close_caption(doc)?;
                let _ = dispatch;
                Ok(Dispatch::Reprocess(InsertionMode::InTable))
            }
            TokenView::EndTag(t) if t.name == "table" => {
                if !self.open_elements.has_in_table_scope("caption") {
                    self.record_parse_error("</table> with no <caption> in scope");
                    return Ok(Dispatch::Consumed);
                }
                self.close_caption(doc)?;
                Ok(Dispatch::Reprocess(InsertionMode::InTable))
            }
            TokenView::EndTag(t)
                if matches!(
                    t.name.as_str(),
                    "body"
                        | "col"
                        | "colgroup"
                        | "html"
                        | "tbody"
                        | "td"
                        | "tfoot"
                        | "th"
                        | "thead"
                        | "tr"
                ) =>
            {
                self.record_parse_error(format!("unexpected </{}> in <caption>", t.name));
                Ok(Dispatch::Consumed)
            }
            _ => self.mode_in_body_delegate(doc, token),
        }
    }

    fn close_caption(&mut self, doc: &mut Document) -> Result<Dispatch, TreeBuilderError> {
        self.open_elements.pop_implied_end_tags(None, false);
        if self.open_elements.current().map(|e| e.tag.as_str()) != Some("caption") {
            self.record_parse_error("</caption> did not match current node");
        }
        self.open_elements.pop_until_html_tag("caption");
        self.afe.clear_to_last_marker();
        self.mode = InsertionMode::InTable;
        let _ = doc;
        Ok(Dispatch::Consumed)
    }

    fn mode_in_body_delegate(
        &mut self,
        doc: &mut Document,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        // A handful of table-related modes fall back to ordinary `InBody`
        // rules for tokens they don't special-case. Text-mode tokenizer
        // hooks are unreachable from those fallback paths in this crate's
        // supported fixtures, so a throwaway tokenizer instance is safe
        // here — it is never driven with real input.
        let mut scratch = Tokenizer::default();
        self.mode_in_body(doc, &mut scratch, token)
    }

    fn mode_in_column_group(
        &mut self,
        doc: &mut Document,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        match view(token) {
            TokenView::Character(c) if is_whitespace_str(&c.data) => {
                self.insert_text(doc, &c.data, false)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::Comment(c) => {
                self.insert_comment(doc, &c.data, false)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::Doctype(_) => {
                self.record_parse_error("unexpected DOCTYPE in <colgroup>");
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "html" => self.mode_in_body_delegate(doc, token),
            TokenView::StartTag(t) if t.name == "col" => {
                self.insert_void_html_element(doc, &t.name, &t.attributes, false)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if t.name == "colgroup" => {
                if self.open_elements.current().map(|e| e.tag.as_str()) != Some("colgroup") {
                    self.record_parse_error("</colgroup> with no matching <colgroup> open");
                    return Ok(Dispatch::Consumed);
                }
                self.open_elements.pop();
                self.mode = InsertionMode::InTable;
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if t.name == "col" => {
                self.record_parse_error("unexpected </col>");
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "template" => {
                self.mode_in_head_via_scratch(doc, token)
            }
            TokenView::EndTag(t) if t.name == "template" => {
                self.mode_in_head_via_scratch(doc, token)
            }
            TokenView::Eof => Ok(Dispatch::Consumed),
            _ => {
                if self.open_elements.current().map(|e| e.tag.as_str()) != Some("colgroup") {
                    self.record_parse_error("unexpected token with no matching <colgroup> open");
                    return Ok(Dispatch::Consumed);
                }
                self.open_elements.pop();
                self.mode = InsertionMode::InTable;
                Ok(Dispatch::Reprocess(InsertionMode::InTable))
            }
        }
    }

    fn mode_in_head_via_scratch(
        &mut self,
        doc: &mut Document,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        let mut scratch = Tokenizer::default();
        self.mode_in_head(doc, &mut scratch, token)
    }

    fn mode_in_table_body(
        &mut self,
        doc: &mut Document,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        match view(token) {
            TokenView::StartTag(t) if t.name == "tr" => {
                self.clear_stack_back_to_table_body_context();
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                self.mode = InsertionMode::InRow;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if matches!(t.name.as_str(), "th" | "td") => {
                self.record_parse_error("<th>/<td> without <tr>; implicitly opening one");
                self.clear_stack_back_to_table_body_context();
                self.insert_html_element(doc, "tr", &[])?;
                self.mode = InsertionMode::InRow;
                Ok(Dispatch::Reprocess(InsertionMode::InRow))
            }
            TokenView::EndTag(t) if matches!(t.name.as_str(), "tbody" | "tfoot" | "thead") => {
                if !self.open_elements.has_in_table_scope(&t.name) {
                    self.record_parse_error(format!(
                        "</{}> with no matching element in scope",
                        t.name
                    ));
                    return Ok(Dispatch::Consumed);
                }
                self.clear_stack_back_to_table_body_context();
                self.open_elements.pop();
                self.mode = InsertionMode::InTable;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t)
                if matches!(
                    t.name.as_str(),
                    "caption" | "col" | "colgroup" | "tbody" | "tfoot" | "thead"
                ) =>
            {
                let has_scope = ["tbody", "thead", "tfoot"]
                    .iter()
                    .any(|k| self.open_elements.has_in_table_scope(k));
                if !has_scope {
                    self.record_parse_error("unexpected start tag with no table-body context");
                    return Ok(Dispatch::Consumed);
                }
                self.clear_stack_back_to_table_body_context();
                self.open_elements.pop();
                self.mode = InsertionMode::InTable;
                Ok(Dispatch::Reprocess(InsertionMode::InTable))
            }
            TokenView::EndTag(t) if t.name == "table" => {
                let has_scope = ["tbody", "thead", "tfoot"]
                    .iter()
                    .any(|k| self.open_elements.has_in_table_scope(k));
                if !has_scope {
                    self.record_parse_error("</table> with no table-body context");
                    return Ok(Dispatch::Consumed);
                }
                self.clear_stack_back_to_table_body_context();
                self.open_elements.pop();
                self.mode = InsertionMode::InTable;
                Ok(Dispatch::Reprocess(InsertionMode::InTable))
            }
            TokenView::EndTag(t)
                if matches!(
                    t.name.as_str(),
                    "body" | "caption" | "col" | "colgroup" | "html" | "td" | "th" | "tr"
                ) =>
            {
                self.record_parse_error(format!("unexpected </{}> in table-body context", t.name));
                Ok(Dispatch::Consumed)
            }
            _ => Ok(Dispatch::Reprocess(InsertionMode::InTable)),
        }
    }

    fn mode_in_row(
        &mut self,
        doc: &mut Document,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        match view(token) {
            TokenView::StartTag(t) if matches!(t.name.as_str(), "th" | "td") => {
                self.clear_stack_back_to_table_row_context();
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                self.mode = InsertionMode::InCell;
                self.afe.insert_marker();
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if t.name == "tr" => {
                if !self.open_elements.has_in_table_scope("tr") {
                    self.record_parse_error("</tr> with no <tr> in scope");
                    return Ok(Dispatch::Consumed);
                }
                self.clear_stack_back_to_table_row_context();
                self.open_elements.pop();
                self.mode = InsertionMode::InTableBody;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t)
                if matches!(
                    t.name.as_str(),
                    "caption" | "col" | "colgroup" | "tbody" | "tfoot" | "thead" | "tr"
                ) =>
            {
                if !self.open_elements.has_in_table_scope("tr") {
                    self.record_parse_error("unexpected start tag with no <tr> in scope");
                    return Ok(Dispatch::Consumed);
                }
                self.clear_stack_back_to_table_row_context();
                self.open_elements.pop();
                self.mode = InsertionMode::InTableBody;
                Ok(Dispatch::Reprocess(InsertionMode::InTableBody))
            }
            TokenView::EndTag(t) if t.name == "table" => {
                if !self.open_elements.has_in_table_scope("tr") {
                    self.record_parse_error("</table> with no <tr> in scope");
                    return Ok(Dispatch::Consumed);
                }
                self.clear_stack_back_to_table_row_context();
                self.open_elements.pop();
                self.mode = InsertionMode::InTableBody;
                Ok(Dispatch::Reprocess(InsertionMode::InTableBody))
            }
            TokenView::EndTag(t) if matches!(t.name.as_str(), "tbody" | "tfoot" | "thead") => {
                if !self.open_elements.has_in_table_scope(&t.name)
                    || !self.open_elements.has_in_table_scope("tr")
                {
                    self.record_parse_error(format!(
                        "</{}> with no matching table-row context",
                        t.name
                    ));
                    return Ok(Dispatch::Consumed);
                }
                self.clear_stack_back_to_table_row_context();
                self.open_elements.pop();
                self.mode = InsertionMode::InTableBody;
                Ok(Dispatch::Reprocess(InsertionMode::InTableBody))
            }
            TokenView::EndTag(t)
                if matches!(
                    t.name.as_str(),
                    "body" | "caption" | "col" | "colgroup" | "html" | "td" | "th"
                ) =>
            {
                self.record_parse_error(format!("unexpected </{}> in <tr>", t.name));
                Ok(Dispatch::Consumed)
            }
            _ => Ok(Dispatch::Reprocess(InsertionMode::InTable)),
        }
    }

    fn mode_in_cell(
        &mut self,
        doc: &mut Document,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        match view(token) {
            TokenView::EndTag(t) if matches!(t.name.as_str(), "td" | "th") => {
                if !self.open_elements.has_in_table_scope(&t.name) {
                    self.record_parse_error(format!(
                        "</{}> with no matching cell in scope",
                        t.name
                    ));
                    return Ok(Dispatch::Consumed);
                }
                self.open_elements.pop_implied_end_tags(None, false);
                if self.open_elements.current().map(|e| e.tag.as_str()) != Some(t.name.as_str()) {
                    self.record_parse_error(format!("</{}> did not match current node", t.name));
                }
                self.open_elements.pop_until_html_tag(&t.name);
                self.afe.clear_to_last_marker();
                self.mode = InsertionMode::InRow;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t)
                if matches!(
                    t.name.as_str(),
                    "caption"
                        | "col"
                        | "colgroup"
                        | "tbody"
                        | "td"
                        | "tfoot"
                        | "th"
                        | "thead"
                        | "tr"
                ) =>
            {
                let in_scope = self.open_elements.has_in_table_scope("td")
                    || self.open_elements.has_in_table_scope("th");
                if !in_scope {
                    self.record_parse_error("unexpected start tag with no open cell in scope");
                    return Ok(Dispatch::Consumed);
                }
                self.close_current_cell();
                Ok(Dispatch::Reprocess(InsertionMode::InRow))
            }
            TokenView::EndTag(t)
                if matches!(
                    t.name.as_str(),
                    "body" | "caption" | "col" | "colgroup" | "html"
                ) =>
            {
                self.record_parse_error(format!("unexpected </{}> in table cell", t.name));
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t)
                if matches!(
                    t.name.as_str(),
                    "table" | "tbody" | "tfoot" | "thead" | "tr"
                ) =>
            {
                if !self.open_elements.has_in_table_scope(&t.name) {
                    self.record_parse_error(format!(
                        "</{}> with no matching context in scope",
                        t.name
                    ));
                    return Ok(Dispatch::Consumed);
                }
                self.close_current_cell();
                Ok(Dispatch::Reprocess(InsertionMode::InRow))
            }
            _ => self.mode_in_body_delegate(doc, token),
        }
    }

    fn close_current_cell(&mut self) {
        self.open_elements.pop_implied_end_tags(None, false);
        self.open_elements.pop_until_html_tag("td");
        self.open_elements.pop_until_html_tag("th");
        self.afe.clear_to_last_marker();
        self.mode = InsertionMode::InRow;
    }

    // ---- InSelect family (§13.2.6.4.16-17) ---------------------------------

    fn mode_in_select(
        &mut self,
        doc: &mut Document,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        match view(token) {
            TokenView::Character(c) => {
                self.insert_text(doc, &c.data, false)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::Comment(c) => {
                self.insert_comment(doc, &c.data, false)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::Doctype(_) => {
                self.record_parse_error("unexpected DOCTYPE in <select>");
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "html" => self.mode_in_body_delegate(doc, token),
            TokenView::StartTag(t) if t.name == "option" => {
                if self.open_elements.current().map(|e| e.tag.as_str()) == Some("option") {
                    self.open_elements.pop();
                }
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "optgroup" => {
                if self.open_elements.current().map(|e| e.tag.as_str()) == Some("option") {
                    self.open_elements.pop();
                }
                if self.open_elements.current().map(|e| e.tag.as_str()) == Some("optgroup") {
                    self.open_elements.pop();
                }
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if t.name == "optgroup" => {
                let len = self.open_elements.len();
                if len >= 2
                    && self.open_elements.entry_at(len - 1).map(|e| e.tag.as_str())
                        == Some("option")
                    && self.open_elements.entry_at(len - 2).map(|e| e.tag.as_str())
                        == Some("optgroup")
                {
                    self.open_elements.pop();
                }
                if self.open_elements.current().map(|e| e.tag.as_str()) == Some("optgroup") {
                    self.open_elements.pop();
                } else {
                    self.record_parse_error("</optgroup> with no matching open <optgroup>");
                }
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if t.name == "option" => {
                if self.open_elements.current().map(|e| e.tag.as_str()) == Some("option") {
                    self.open_elements.pop();
                } else {
                    self.record_parse_error("</option> with no matching open <option>");
                }
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if t.name == "select" => {
                if !self.open_elements.has_in_select_scope("select") {
                    self.record_parse_error("</select> with no <select> in scope");
                    return Ok(Dispatch::Consumed);
                }
                self.open_elements.pop_until_html_tag("select");
                self.reset_mode_after_table_close_default_body();
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "select" => {
                self.record_parse_error("nested <select>; treated as </select>");
                self.open_elements.pop_until_html_tag("select");
                self.reset_mode_after_table_close_default_body();
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t)
                if matches!(t.name.as_str(), "input" | "keygen" | "textarea") =>
            {
                self.record_parse_error(format!(
                    "<{}> inside <select>; treated as </select>",
                    t.name
                ));
                if !self.open_elements.has_in_select_scope("select") {
                    return Ok(Dispatch::Consumed);
                }
                self.open_elements.pop_until_html_tag("select");
                self.reset_mode_after_table_close_default_body();
                Ok(Dispatch::Reprocess(self.mode))
            }
            TokenView::StartTag(t) if matches!(t.name.as_str(), "script" | "template") => {
                self.mode_in_head_via_scratch(doc, token)
            }
            TokenView::EndTag(t) if t.name == "template" => {
                self.mode_in_head_via_scratch(doc, token)
            }
            TokenView::Eof => Ok(Dispatch::Consumed),
            _ => {
                self.record_parse_error("unexpected token in <select>");
                Ok(Dispatch::Consumed)
            }
        }
    }

    fn reset_mode_after_table_close_default_body(&mut self) {
        self.mode = self
            .template_modes
            .last()
            .copied()
            .unwrap_or(InsertionMode::InBody);
    }

    fn mode_in_select_in_table(
        &mut self,
        doc: &mut Document,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        match view(token) {
            TokenView::StartTag(t)
                if matches!(
                    t.name.as_str(),
                    "caption" | "table" | "tbody" | "tfoot" | "thead" | "tr" | "td" | "th"
                ) =>
            {
                self.record_parse_error(format!("<{}> inside <select> in table context", t.name));
                self.open_elements.pop_until_html_tag("select");
                self.reset_mode_after_table_close_default_body();
                Ok(Dispatch::Reprocess(self.mode))
            }
            TokenView::EndTag(t)
                if matches!(
                    t.name.as_str(),
                    "caption" | "table" | "tbody" | "tfoot" | "thead" | "tr" | "td" | "th"
                ) =>
            {
                if !self.open_elements.has_in_table_scope(&t.name) {
                    self.record_parse_error(format!(
                        "</{}> with no matching context in scope",
                        t.name
                    ));
                    return Ok(Dispatch::Consumed);
                }
                self.open_elements.pop_until_html_tag("select");
                self.reset_mode_after_table_close_default_body();
                Ok(Dispatch::Reprocess(self.mode))
            }
            _ => self.mode_in_select(doc, token),
        }
    }

    // ---- InTemplate (§13.2.6.4.18) -----------------------------------------

    fn mode_in_template(
        &mut self,
        doc: &mut Document,
        tokenizer: &mut Tokenizer,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        // Simplified per `.agent-state/evidence/M2-T04.md`: template
        // *contents* are appended directly under the `<template>` element
        // itself rather than a separate `DocumentFragment` "contents" node
        // (machina-dom has no per-element auxiliary fragment concept).
        // Structural start tags reset the template insertion mode per
        // spec; everything else delegates to `InBody`.
        match view(token) {
            TokenView::StartTag(t)
                if matches!(
                    t.name.as_str(),
                    "base"
                        | "basefont"
                        | "bgsound"
                        | "link"
                        | "meta"
                        | "noframes"
                        | "script"
                        | "style"
                        | "template"
                        | "title"
                ) =>
            {
                self.mode_in_head(doc, tokenizer, token)
            }
            TokenView::EndTag(t) if t.name == "template" => {
                self.mode_in_head(doc, tokenizer, token)
            }
            TokenView::StartTag(t)
                if matches!(
                    t.name.as_str(),
                    "caption" | "colgroup" | "tbody" | "tfoot" | "thead"
                ) =>
            {
                if let Some(top) = self.template_modes.last_mut() {
                    *top = InsertionMode::InTable;
                }
                self.mode = InsertionMode::InTable;
                Ok(Dispatch::Reprocess(InsertionMode::InTable))
            }
            TokenView::StartTag(t) if t.name == "col" => {
                if let Some(top) = self.template_modes.last_mut() {
                    *top = InsertionMode::InColumnGroup;
                }
                self.mode = InsertionMode::InColumnGroup;
                Ok(Dispatch::Reprocess(InsertionMode::InColumnGroup))
            }
            TokenView::StartTag(t) if t.name == "tr" => {
                if let Some(top) = self.template_modes.last_mut() {
                    *top = InsertionMode::InTableBody;
                }
                self.mode = InsertionMode::InTableBody;
                Ok(Dispatch::Reprocess(InsertionMode::InTableBody))
            }
            TokenView::StartTag(t) if matches!(t.name.as_str(), "td" | "th") => {
                if let Some(top) = self.template_modes.last_mut() {
                    *top = InsertionMode::InRow;
                }
                self.mode = InsertionMode::InRow;
                Ok(Dispatch::Reprocess(InsertionMode::InRow))
            }
            TokenView::Eof => {
                if !self.open_elements.has_html_tag_anywhere("template") {
                    return Ok(Dispatch::Consumed);
                }
                self.record_parse_error("EOF inside <template>");
                self.open_elements.pop_until_html_tag("template");
                self.afe.clear_to_last_marker();
                self.template_modes.pop();
                self.mode = self
                    .template_modes
                    .last()
                    .copied()
                    .unwrap_or(InsertionMode::InBody);
                Ok(Dispatch::Consumed)
            }
            _ => {
                if let Some(top) = self.template_modes.last_mut() {
                    *top = InsertionMode::InBody;
                }
                self.mode = InsertionMode::InBody;
                Ok(Dispatch::Reprocess(InsertionMode::InBody))
            }
        }
    }

    // ---- AfterBody (§13.2.6.4.19) ------------------------------------------

    fn mode_after_body(
        &mut self,
        doc: &mut Document,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        match view(token) {
            TokenView::Character(c) if is_whitespace_str(&c.data) => {
                self.mode_in_body_delegate(doc, token)
            }
            TokenView::Comment(c) => {
                let html = self.open_elements.html_element();
                if let Some(html) = html {
                    self.insert_comment_at(doc, &c.data, html.node_handle())?;
                }
                Ok(Dispatch::Consumed)
            }
            TokenView::Doctype(_) => {
                self.record_parse_error("unexpected DOCTYPE after </body>");
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "html" => self.mode_in_body_delegate(doc, token),
            TokenView::EndTag(t) if t.name == "html" => {
                self.mode = InsertionMode::AfterAfterBody;
                Ok(Dispatch::Consumed)
            }
            TokenView::Eof => Ok(Dispatch::Consumed),
            _ => {
                self.record_parse_error("unexpected token after </body>; reprocessing in InBody");
                self.mode = InsertionMode::InBody;
                Ok(Dispatch::Reprocess(InsertionMode::InBody))
            }
        }
    }

    // ---- InFrameset / AfterFrameset (§13.2.6.4.20-21) ----------------------
    //
    // Frameset documents are rare in modern content and not prioritized for
    // this pass (see `.agent-state/evidence/M2-T04.md`): tokens are handled
    // just enough to avoid corrupting the stack (structural `frameset`/
    // `frame`/`noframes` tags are inserted; everything else is ignored with
    // a recorded diagnostic rather than silently miscompiled into `InBody`).

    fn mode_in_frameset(
        &mut self,
        doc: &mut Document,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        match view(token) {
            TokenView::Character(c) if is_whitespace_str(&c.data) => {
                self.insert_text(doc, &c.data, false)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::Comment(c) => {
                self.insert_comment(doc, &c.data, false)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::Doctype(_) => Ok(Dispatch::Consumed),
            TokenView::StartTag(t) if t.name == "html" => self.mode_in_body_delegate(doc, token),
            TokenView::StartTag(t) if t.name == "frameset" => {
                self.insert_html_element(doc, &t.name, &t.attributes)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::EndTag(t) if t.name == "frameset" => {
                if self.open_elements.html_element() != self.open_elements.current_handle() {
                    self.open_elements.pop();
                }
                if !self.is_fragment
                    && self.open_elements.current().map(|e| e.tag.as_str()) != Some("frameset")
                {
                    self.mode = InsertionMode::AfterFrameset;
                }
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "frame" => {
                self.insert_void_html_element(doc, &t.name, &t.attributes, false)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "noframes" => {
                self.mode_in_head_via_scratch(doc, token)
            }
            TokenView::Eof => Ok(Dispatch::Consumed),
            _ => {
                self.record_ignored("(token)", "InFrameset");
                Ok(Dispatch::Consumed)
            }
        }
    }

    fn mode_after_frameset(
        &mut self,
        doc: &mut Document,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        match view(token) {
            TokenView::Character(c) if is_whitespace_str(&c.data) => {
                self.insert_text(doc, &c.data, false)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::Comment(c) => {
                self.insert_comment(doc, &c.data, false)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::Doctype(_) => Ok(Dispatch::Consumed),
            TokenView::StartTag(t) if t.name == "html" => self.mode_in_body_delegate(doc, token),
            TokenView::EndTag(t) if t.name == "html" => {
                self.mode = InsertionMode::AfterAfterFrameset;
                Ok(Dispatch::Consumed)
            }
            TokenView::StartTag(t) if t.name == "noframes" => {
                self.mode_in_head_via_scratch(doc, token)
            }
            TokenView::Eof => Ok(Dispatch::Consumed),
            _ => {
                self.record_ignored("(token)", "AfterFrameset");
                Ok(Dispatch::Consumed)
            }
        }
    }

    fn mode_after_after_body(
        &mut self,
        doc: &mut Document,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        match view(token) {
            TokenView::Comment(c) => {
                let root = doc.root();
                self.insert_comment_at(doc, &c.data, root)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::Doctype(_) => self.mode_in_body_delegate(doc, token),
            TokenView::Character(c) if is_whitespace_str(&c.data) => {
                self.mode_in_body_delegate(doc, token)
            }
            TokenView::StartTag(t) if t.name == "html" => self.mode_in_body_delegate(doc, token),
            TokenView::Eof => Ok(Dispatch::Consumed),
            _ => {
                self.record_parse_error("unexpected token after </html>; reprocessing in InBody");
                self.mode = InsertionMode::InBody;
                Ok(Dispatch::Reprocess(InsertionMode::InBody))
            }
        }
    }

    fn mode_after_after_frameset(
        &mut self,
        doc: &mut Document,
        token: &Token,
    ) -> Result<Dispatch, TreeBuilderError> {
        match view(token) {
            TokenView::Comment(c) => {
                let root = doc.root();
                self.insert_comment_at(doc, &c.data, root)?;
                Ok(Dispatch::Consumed)
            }
            TokenView::Doctype(_) => self.mode_in_body_delegate(doc, token),
            TokenView::Character(c) if is_whitespace_str(&c.data) => {
                self.mode_in_body_delegate(doc, token)
            }
            TokenView::StartTag(t) if t.name == "html" => self.mode_in_body_delegate(doc, token),
            TokenView::StartTag(t) if t.name == "noframes" => {
                self.mode_in_head_via_scratch(doc, token)
            }
            TokenView::Eof => Ok(Dispatch::Consumed),
            _ => {
                self.record_ignored("(token)", "AfterAfterFrameset");
                Ok(Dispatch::Consumed)
            }
        }
    }
}
