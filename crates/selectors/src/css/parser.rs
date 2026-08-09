//! Recursive-descent CSS selector parser: token stream → immutable
//! [`SelectorList`] AST. `Result` at every production; a syntax error is
//! always [`QueryError::InvalidSelector`] and a valid-but-out-of-scope
//! construct is always [`QueryError::UnsupportedFeature`] — never conflated
//! (design §6).

use crate::css::ast::{
    AttrOperator, Combinator, CompoundSelector, NthExpr, PseudoClass, SelectorList, SimpleSelector,
};
use crate::css::pseudo::parse_nth;
use crate::css::tokenizer::{tokenize, PositionedToken, Token};
use crate::error::QueryError;
use crate::limits::MAX_NESTING_DEPTH;

/// Pseudo-classes that are real, valid CSS but explicitly out of this
/// crate's current matching scope (design §1's "explicitly deferred, not
/// silently unsupported" list). Matching one of these names always produces
/// [`QueryError::UnsupportedFeature`], never [`QueryError::InvalidSelector`]
/// and never a silent no-op/empty match.
const KNOWN_UNSUPPORTED_PSEUDO_CLASSES: &[&str] = &[
    "hover",
    "focus",
    "active",
    "visited",
    "target",
    "lang",
    "has",
    "is",
    "where",
    "first-of-type",
    "last-of-type",
    "nth-of-type",
    "nth-last-of-type",
    "only-of-type",
    "link",
    "any-link",
    "checked",
    "disabled",
    "enabled",
    "required",
    "optional",
    "read-only",
    "read-write",
    "placeholder-shown",
    "focus-within",
    "focus-visible",
    "defined",
    "dir",
    "before",
    "after",
    "host",
    "host-context",
    "scope",
    "default",
    "indeterminate",
    "invalid",
    "valid",
    "out-of-range",
    "in-range",
    "first",
    "left",
    "right",
];

struct Parser {
    tokens: Vec<PositionedToken>,
    cursor: usize,
    source_len: usize,
    /// Depth of `:not(...)` nesting currently being parsed; bounded by
    /// [`MAX_NESTING_DEPTH`] so `:not(:not(:not(...)))` fails closed instead
    /// of blowing the parser's own call stack (design §6, `TooComplex`).
    not_depth: usize,
}

pub(crate) fn parse_selector_list(source: &str) -> Result<SelectorList, QueryError> {
    let tokens = tokenize(source)?;
    let mut parser = Parser {
        tokens,
        cursor: 0,
        source_len: source.len(),
        not_depth: 0,
    };
    let list = parser.parse_selector_list()?;
    parser.skip_whitespace();
    if !matches!(parser.peek(), Token::Eof) {
        return Err(QueryError::InvalidSelector {
            message: "unexpected trailing content after selector".to_string(),
            position: parser.position(),
        });
    }
    Ok(list)
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.cursor].token
    }

    fn position(&self) -> usize {
        self.tokens[self.cursor].position
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.cursor].token.clone();
        if self.cursor + 1 < self.tokens.len() {
            self.cursor += 1;
        }
        token
    }

    fn skip_whitespace(&mut self) -> bool {
        let mut saw = false;
        while matches!(self.peek(), Token::Whitespace) {
            saw = true;
            self.advance();
        }
        saw
    }

    fn parse_selector_list(&mut self) -> Result<SelectorList, QueryError> {
        self.skip_whitespace();
        let mut selectors = vec![self.parse_complex_selector()?];
        loop {
            self.skip_whitespace();
            if matches!(self.peek(), Token::Comma) {
                self.advance();
                self.skip_whitespace();
                selectors.push(self.parse_complex_selector()?);
            } else {
                break;
            }
        }
        Ok(SelectorList { selectors })
    }

    fn parse_complex_selector(&mut self) -> Result<crate::css::ast::ComplexSelector, QueryError> {
        let mut compounds = vec![self.parse_compound_selector()?];
        let mut combinators = Vec::new();
        loop {
            let saw_ws = self.skip_whitespace();
            let combinator = match self.peek() {
                Token::Gt => {
                    self.advance();
                    self.skip_whitespace();
                    Some(Combinator::Child)
                }
                Token::Plus => {
                    self.advance();
                    self.skip_whitespace();
                    Some(Combinator::AdjacentSibling)
                }
                Token::Tilde => {
                    self.advance();
                    self.skip_whitespace();
                    Some(Combinator::GeneralSibling)
                }
                Token::Comma | Token::Eof | Token::RParen => None,
                _ if saw_ws => Some(Combinator::Descendant),
                _ => {
                    return Err(QueryError::InvalidSelector {
                        message: "expected combinator or end of selector".to_string(),
                        position: self.position(),
                    });
                }
            };
            match combinator {
                Some(c) => {
                    combinators.push(c);
                    if combinators.len() > MAX_NESTING_DEPTH {
                        return Err(QueryError::TooComplex {
                            limit: crate::error::LimitKind::SelectorNesting,
                        });
                    }
                    compounds.push(self.parse_compound_selector()?);
                }
                None => break,
            }
        }
        Ok(crate::css::ast::ComplexSelector {
            compounds,
            combinators,
        })
    }

    fn parse_compound_selector(&mut self) -> Result<CompoundSelector, QueryError> {
        let mut simple_selectors = Vec::new();
        loop {
            match self.peek().clone() {
                Token::Star => {
                    self.advance();
                    simple_selectors.push(SimpleSelector::Universal);
                }
                Token::Ident(name) => {
                    self.advance();
                    simple_selectors.push(SimpleSelector::Type(name.to_ascii_lowercase()));
                }
                Token::Dot => {
                    self.advance();
                    match self.peek().clone() {
                        Token::Ident(name) => {
                            self.advance();
                            simple_selectors.push(SimpleSelector::Class(name));
                        }
                        _ => {
                            return Err(QueryError::InvalidSelector {
                                message: "expected class name after '.'".to_string(),
                                position: self.position(),
                            });
                        }
                    }
                }
                Token::Hash(name) => {
                    self.advance();
                    simple_selectors.push(SimpleSelector::Id(name));
                }
                Token::LBracket => {
                    simple_selectors.push(self.parse_attribute_selector()?);
                }
                Token::Colon => {
                    simple_selectors.push(self.parse_pseudo_class()?);
                }
                Token::ColonColon => {
                    return Err(QueryError::UnsupportedFeature {
                        feature: "pseudo-elements (::before, ::after, ...)".to_string(),
                        position: self.position(),
                    });
                }
                _ => break,
            }
        }
        if simple_selectors.is_empty() {
            return Err(QueryError::InvalidSelector {
                message: "expected a selector".to_string(),
                position: self.position(),
            });
        }
        simple_selectors.sort_by_key(|s| s.match_cost_rank());
        Ok(CompoundSelector { simple_selectors })
    }

    fn parse_attribute_selector(&mut self) -> Result<SimpleSelector, QueryError> {
        let open_position = self.position();
        self.advance(); // consume '['
        self.skip_whitespace();
        let name = match self.peek().clone() {
            Token::Ident(name) => {
                self.advance();
                name
            }
            _ => {
                return Err(QueryError::InvalidSelector {
                    message: "expected attribute name after '['".to_string(),
                    position: self.position(),
                });
            }
        };
        self.skip_whitespace();
        let operator = match self.peek().clone() {
            Token::RBracket => {
                self.advance();
                return Ok(SimpleSelector::Attribute {
                    name,
                    operator: AttrOperator::Present,
                });
            }
            Token::Eq => {
                self.advance();
                AttrOperator::Equals(self.parse_attr_value()?)
            }
            Token::IncludeMatch => {
                self.advance();
                AttrOperator::Includes(self.parse_attr_value()?)
            }
            Token::DashMatch => {
                self.advance();
                AttrOperator::DashMatch(self.parse_attr_value()?)
            }
            Token::PrefixMatch => {
                self.advance();
                AttrOperator::PrefixMatch(self.parse_attr_value()?)
            }
            Token::SuffixMatch => {
                self.advance();
                AttrOperator::SuffixMatch(self.parse_attr_value()?)
            }
            Token::SubstringMatch => {
                self.advance();
                AttrOperator::SubstringMatch(self.parse_attr_value()?)
            }
            _ => {
                return Err(QueryError::InvalidSelector {
                    message: "expected an attribute operator or ']'".to_string(),
                    position: self.position(),
                });
            }
        };
        self.skip_whitespace();
        if let Token::Ident(flag) = self.peek().clone() {
            if flag.eq_ignore_ascii_case("i") || flag.eq_ignore_ascii_case("s") {
                return Err(QueryError::UnsupportedFeature {
                    feature: "attribute case-sensitivity flag ('i'/'s')".to_string(),
                    position: self.position(),
                });
            }
        }
        match self.peek() {
            Token::RBracket => {
                self.advance();
                Ok(SimpleSelector::Attribute { name, operator })
            }
            _ => Err(QueryError::InvalidSelector {
                message: "expected ']' to close attribute selector".to_string(),
                position: open_position,
            }),
        }
    }

    fn parse_attr_value(&mut self) -> Result<String, QueryError> {
        self.skip_whitespace();
        match self.peek().clone() {
            Token::Str(value) => {
                self.advance();
                Ok(value)
            }
            Token::Ident(value) => {
                self.advance();
                Ok(value)
            }
            _ => Err(QueryError::InvalidSelector {
                message: "expected an attribute value (string or identifier)".to_string(),
                position: self.position(),
            }),
        }
    }

    fn parse_pseudo_class(&mut self) -> Result<SimpleSelector, QueryError> {
        let colon_position = self.position();
        self.advance(); // consume ':'
        let name = match self.peek().clone() {
            Token::Ident(name) => {
                self.advance();
                name
            }
            _ => {
                return Err(QueryError::InvalidSelector {
                    message: "expected pseudo-class name after ':'".to_string(),
                    position: self.position(),
                });
            }
        };
        let lower = name.to_ascii_lowercase();
        match lower.as_str() {
            "first-child" => Ok(SimpleSelector::PseudoClass(PseudoClass::FirstChild)),
            "last-child" => Ok(SimpleSelector::PseudoClass(PseudoClass::LastChild)),
            "only-child" => Ok(SimpleSelector::PseudoClass(PseudoClass::OnlyChild)),
            "empty" => Ok(SimpleSelector::PseudoClass(PseudoClass::Empty)),
            "root" => Ok(SimpleSelector::PseudoClass(PseudoClass::Root)),
            "not" => {
                self.not_depth += 1;
                if self.not_depth > MAX_NESTING_DEPTH {
                    return Err(QueryError::TooComplex {
                        limit: crate::error::LimitKind::SelectorNesting,
                    });
                }
                self.expect_lparen()?;
                self.skip_whitespace();
                let inner = self.parse_compound_selector()?;
                self.skip_whitespace();
                self.expect_rparen()?;
                self.not_depth -= 1;
                Ok(SimpleSelector::Negation(Box::new(inner)))
            }
            "nth-child" => {
                let expr = self.parse_nth_argument()?;
                Ok(SimpleSelector::PseudoClass(PseudoClass::NthChild(expr)))
            }
            "nth-last-child" => {
                let expr = self.parse_nth_argument()?;
                Ok(SimpleSelector::PseudoClass(PseudoClass::NthLastChild(expr)))
            }
            other if KNOWN_UNSUPPORTED_PSEUDO_CLASSES.contains(&other) => {
                // A valid CSS pseudo-class name that this crate does not
                // implement yet. If it takes an argument list, consume the
                // parenthesized content too (so a later unrelated token
                // isn't misparsed) without interpreting it.
                if matches!(self.peek(), Token::LParen) {
                    self.skip_balanced_parens()?;
                }
                Err(QueryError::UnsupportedFeature {
                    feature: format!(":{other}"),
                    position: colon_position,
                })
            }
            _ => Err(QueryError::InvalidSelector {
                message: format!("unknown pseudo-class ':{name}'"),
                position: colon_position,
            }),
        }
    }

    fn expect_lparen(&mut self) -> Result<(), QueryError> {
        match self.peek() {
            Token::LParen => {
                self.advance();
                Ok(())
            }
            _ => Err(QueryError::InvalidSelector {
                message: "expected '(' after pseudo-class name".to_string(),
                position: self.position(),
            }),
        }
    }

    fn expect_rparen(&mut self) -> Result<(), QueryError> {
        match self.peek() {
            Token::RParen => {
                self.advance();
                Ok(())
            }
            _ => Err(QueryError::InvalidSelector {
                message: "unterminated pseudo-class argument list (expected ')')".to_string(),
                position: self.position(),
            }),
        }
    }

    fn skip_balanced_parens(&mut self) -> Result<(), QueryError> {
        self.expect_lparen()?;
        let mut depth = 1usize;
        loop {
            match self.peek() {
                Token::LParen => {
                    depth += 1;
                    self.advance();
                }
                Token::RParen => {
                    depth -= 1;
                    self.advance();
                    if depth == 0 {
                        return Ok(());
                    }
                }
                Token::Eof => {
                    return Err(QueryError::InvalidSelector {
                        message: "unterminated pseudo-class argument list".to_string(),
                        position: self.position(),
                    });
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    /// Extracts the raw source text between `nth-child(`/`nth-last-child(`
    /// and its matching `)` and hands it to [`parse_nth`], rather than
    /// forcing the general tokenizer to model the `an+b` dimension grammar
    /// (see `css/pseudo.rs`'s module docs).
    fn parse_nth_argument(&mut self) -> Result<NthExpr, QueryError> {
        self.expect_lparen()?;
        let content_start = self.position();
        let mut content_end = self.source_len;
        let mut found = false;
        while !matches!(self.peek(), Token::Eof) {
            if matches!(self.peek(), Token::RParen) {
                content_end = self.position();
                found = true;
                break;
            }
            self.advance();
        }
        if !found {
            return Err(QueryError::InvalidSelector {
                message: "unterminated :nth-child(...)".to_string(),
                position: content_start,
            });
        }
        // `content_end` is the RParen's own position; recover the raw slice
        // from the *original* token positions rather than re-deriving from
        // token text, since Whitespace tokens don't carry their consumed
        // text.
        let raw = self.raw_between(content_start, content_end);
        self.expect_rparen()?;
        parse_nth(&raw, content_start)
    }

    fn raw_between(&self, _start: usize, _end: usize) -> String {
        // Reconstructed from tokens (not a direct source slice) since the
        // tokenizer does not retain the original `&str` after this point;
        // whitespace tokens are re-emitted as a single space, which
        // `parse_nth` already normalizes away.
        let mut out = String::new();
        for positioned in &self.tokens {
            if positioned.position < _start || positioned.position >= _end {
                continue;
            }
            match &positioned.token {
                Token::Whitespace => out.push(' '),
                Token::Ident(s) => out.push_str(s),
                Token::Plus => out.push('+'),
                Token::Gt => out.push('>'),
                Token::Tilde => out.push('~'),
                _ => {}
            }
        }
        out
    }
}
