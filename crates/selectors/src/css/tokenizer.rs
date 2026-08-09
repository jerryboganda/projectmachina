//! Hand-written CSS selector lexer: turns selector text into a flat token
//! stream with byte-offset positions before any grammar composition begins
//! (two-phase, mirrors `crates/html-tree-builder`'s tokenizer/parser split
//! and M2-T05's validate-then-commit posture — a lexical error can never
//! leave a partially-built AST). Whitespace is a significant, explicit
//! token (not skipped) since a bare run of whitespace between compound
//! selectors is itself the descendant combinator.

use crate::error::QueryError;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Token {
    Ident(String),
    Hash(String),
    Str(String),
    Whitespace,
    Dot,
    Colon,
    ColonColon,
    Comma,
    Star,
    LBracket,
    RBracket,
    LParen,
    RParen,
    Gt,
    Plus,
    Tilde,
    Pipe,
    Eq,
    IncludeMatch,
    DashMatch,
    PrefixMatch,
    SuffixMatch,
    SubstringMatch,
    Eof,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PositionedToken {
    pub(crate) token: Token,
    pub(crate) position: usize,
}

/// Deliberately broader than the CSS spec's own `ident-start-code-point`
/// (which excludes ASCII digits): the `an+b` micro-syntax inside
/// `:nth-child(...)` needs digit-leading tokens ("2n+1", "3", "-1") to
/// tokenize as ident-like blobs so `css/pseudo.rs::parse_nth` can
/// reconstruct and interpret them (see that module's docs) without this
/// tokenizer needing a separate numeric-dimension token type. This does not
/// change matching correctness elsewhere: a digit-leading token used as a
/// type/class/id selector simply never matches a real HTML tag/attribute
/// name (HTML names never start with a digit), so it is a harmless
/// over-acceptance, not a silent-mismatch hazard.
fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-' || (!c.is_ascii() && !c.is_control())
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-' || (!c.is_ascii() && !c.is_control())
}

pub(crate) fn tokenize(source: &str) -> Result<Vec<PositionedToken>, QueryError> {
    let mut tokens = Vec::new();
    let bytes = source.as_bytes();
    let mut chars = source.char_indices().peekable();

    while let Some(&(start, c)) = chars.peek() {
        if c.is_whitespace() {
            while matches!(chars.peek(), Some(&(_, c)) if c.is_whitespace()) {
                chars.next();
            }
            tokens.push(PositionedToken {
                token: Token::Whitespace,
                position: start,
            });
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
        match c {
            '#' => {
                chars.next();
                let name_start = match chars.peek() {
                    Some(&(pos, c2)) if is_ident_start(c2) => pos,
                    _ => {
                        return Err(QueryError::InvalidSelector {
                            message: "'#' must be followed by an identifier".to_string(),
                            position: start,
                        });
                    }
                };
                let mut end = name_start;
                while let Some(&(pos, c2)) = chars.peek() {
                    if is_ident_continue(c2) {
                        end = pos + c2.len_utf8();
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(PositionedToken {
                    token: Token::Hash(source[name_start..end].to_string()),
                    position: start,
                });
            }
            '.' => {
                chars.next();
                tokens.push(PositionedToken {
                    token: Token::Dot,
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
                    tokens.push(PositionedToken {
                        token: Token::Colon,
                        position: start,
                    });
                }
            }
            ',' => {
                chars.next();
                tokens.push(PositionedToken {
                    token: Token::Comma,
                    position: start,
                });
            }
            '*' => {
                chars.next();
                if matches!(chars.peek(), Some(&(_, '='))) {
                    chars.next();
                    tokens.push(PositionedToken {
                        token: Token::SubstringMatch,
                        position: start,
                    });
                } else {
                    tokens.push(PositionedToken {
                        token: Token::Star,
                        position: start,
                    });
                }
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
            '>' => {
                chars.next();
                tokens.push(PositionedToken {
                    token: Token::Gt,
                    position: start,
                });
            }
            '+' => {
                chars.next();
                tokens.push(PositionedToken {
                    token: Token::Plus,
                    position: start,
                });
            }
            '~' => {
                chars.next();
                if matches!(chars.peek(), Some(&(_, '='))) {
                    chars.next();
                    tokens.push(PositionedToken {
                        token: Token::IncludeMatch,
                        position: start,
                    });
                } else {
                    tokens.push(PositionedToken {
                        token: Token::Tilde,
                        position: start,
                    });
                }
            }
            '|' => {
                chars.next();
                if matches!(chars.peek(), Some(&(_, '='))) {
                    chars.next();
                    tokens.push(PositionedToken {
                        token: Token::DashMatch,
                        position: start,
                    });
                } else {
                    tokens.push(PositionedToken {
                        token: Token::Pipe,
                        position: start,
                    });
                }
            }
            '=' => {
                chars.next();
                tokens.push(PositionedToken {
                    token: Token::Eq,
                    position: start,
                });
            }
            '^' => {
                chars.next();
                if matches!(chars.peek(), Some(&(_, '='))) {
                    chars.next();
                    tokens.push(PositionedToken {
                        token: Token::PrefixMatch,
                        position: start,
                    });
                } else {
                    return Err(QueryError::InvalidSelector {
                        message: "'^' must be followed by '=' (prefix-match operator)".to_string(),
                        position: start,
                    });
                }
            }
            '$' => {
                chars.next();
                if matches!(chars.peek(), Some(&(_, '='))) {
                    chars.next();
                    tokens.push(PositionedToken {
                        token: Token::SuffixMatch,
                        position: start,
                    });
                } else {
                    return Err(QueryError::InvalidSelector {
                        message: "'$' must be followed by '=' (suffix-match operator)".to_string(),
                        position: start,
                    });
                }
            }
            '"' | '\'' => {
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
                    if c2 == '\\' {
                        if let Some(&(_, escaped)) = chars.peek() {
                            value.push(escaped);
                            chars.next();
                            continue;
                        }
                        break;
                    }
                    value.push(c2);
                }
                if !terminated {
                    return Err(QueryError::InvalidSelector {
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
                return Err(QueryError::InvalidSelector {
                    message: format!("unexpected character '{other}'"),
                    position: start,
                });
            }
        }
    }

    tokens.push(PositionedToken {
        token: Token::Eof,
        position: bytes.len(),
    });
    Ok(tokens)
}
