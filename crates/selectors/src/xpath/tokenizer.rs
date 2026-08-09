//! Hand-written XPath lexer: text → flat token stream with byte-offset
//! positions, staged strictly before grammar composition (same two-phase
//! posture as `css/tokenizer.rs`).

use crate::error::QueryError;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Token {
    Slash,
    SlashSlash,
    Dot,
    DotDot,
    At,
    ColonColon,
    Ident(String),
    Number(String),
    Star,
    LBracket,
    RBracket,
    LParen,
    RParen,
    Eq,
    /// Not used by any construct in this crate's grammar subset (design
    /// §5): tokenized anyway so a multi-argument unsupported function call
    /// like `contains(text(), 'x')` fails via the parser's
    /// `QueryError::UnsupportedFeature` classification (design §6) instead
    /// of an unrelated tokenizer-level `InvalidXPath` on the comma
    /// character — the comma is genuine, valid XPath syntax, just for a
    /// function this crate does not implement.
    Comma,
    Str(String),
    Eof,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PositionedToken {
    pub(crate) token: Token,
    pub(crate) position: usize,
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || (!c.is_ascii() && !c.is_control())
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-' || (!c.is_ascii() && !c.is_control())
}

pub(crate) fn tokenize(source: &str) -> Result<Vec<PositionedToken>, QueryError> {
    let mut tokens = Vec::new();
    let mut chars = source.char_indices().peekable();

    while let Some(&(start, c)) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        if is_ident_start(c) {
            let mut end = start + c.len_utf8();
            chars.next();
            while let Some(&(pos, c2)) = chars.peek() {
                if is_ident_continue(c2) {
                    end = pos + c2.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }
            tokens.push(PositionedToken {
                token: Token::Ident(source[start..end].to_string()),
                position: start,
            });
            continue;
        }
        if c.is_ascii_digit() {
            let mut end = start + 1;
            chars.next();
            while let Some(&(pos, c2)) = chars.peek() {
                if c2.is_ascii_digit() {
                    end = pos + c2.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }
            tokens.push(PositionedToken {
                token: Token::Number(source[start..end].to_string()),
                position: start,
            });
            continue;
        }
        match c {
            '/' => {
                chars.next();
                if matches!(chars.peek(), Some(&(_, '/'))) {
                    chars.next();
                    tokens.push(PositionedToken {
                        token: Token::SlashSlash,
                        position: start,
                    });
                } else {
                    tokens.push(PositionedToken {
                        token: Token::Slash,
                        position: start,
                    });
                }
            }
            '.' => {
                chars.next();
                if matches!(chars.peek(), Some(&(_, '.'))) {
                    chars.next();
                    tokens.push(PositionedToken {
                        token: Token::DotDot,
                        position: start,
                    });
                } else {
                    tokens.push(PositionedToken {
                        token: Token::Dot,
                        position: start,
                    });
                }
            }
            '@' => {
                chars.next();
                tokens.push(PositionedToken {
                    token: Token::At,
                    position: start,
                });
            }
            ':' => {
                chars.next();
                if matches!(chars.peek(), Some(&(_, ':'))) {
                    chars.next();
                    tokens.push(PositionedToken {
                        token: Token::ColonColon,
                        position: start,
                    });
                } else {
                    return Err(QueryError::InvalidXPath {
                        message: "unexpected single ':' (expected '::')".to_string(),
                        position: start,
                    });
                }
            }
            '*' => {
                chars.next();
                tokens.push(PositionedToken {
                    token: Token::Star,
                    position: start,
                });
            }
            '[' => {
                chars.next();
                tokens.push(PositionedToken {
                    token: Token::LBracket,
                    position: start,
                });
            }
            ']' => {
                chars.next();
                tokens.push(PositionedToken {
                    token: Token::RBracket,
                    position: start,
                });
            }
            '(' => {
                chars.next();
                tokens.push(PositionedToken {
                    token: Token::LParen,
                    position: start,
                });
            }
            ')' => {
                chars.next();
                tokens.push(PositionedToken {
                    token: Token::RParen,
                    position: start,
                });
            }
            '=' => {
                chars.next();
                tokens.push(PositionedToken {
                    token: Token::Eq,
                    position: start,
                });
            }
            ',' => {
                chars.next();
                tokens.push(PositionedToken {
                    token: Token::Comma,
                    position: start,
                });
            }
            '\'' | '"' => {
                let quote = c;
                chars.next();
                let mut value = String::new();
                let mut terminated = false;
                while let Some(&(_, c2)) = chars.peek() {
                    chars.next();
                    if c2 == quote {
                        terminated = true;
                        break;
                    }
                    value.push(c2);
                }
                if !terminated {
                    return Err(QueryError::InvalidXPath {
                        message: "unterminated string literal".to_string(),
                        position: start,
                    });
                }
                tokens.push(PositionedToken {
                    token: Token::Str(value),
                    position: start,
                });
            }
            other => {
                return Err(QueryError::InvalidXPath {
                    message: format!("unexpected character '{other}'"),
                    position: start,
                });
            }
        }
    }

    tokens.push(PositionedToken {
        token: Token::Eof,
        position: source.len(),
    });
    Ok(tokens)
}
