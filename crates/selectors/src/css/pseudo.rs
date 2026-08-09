//! `:nth-child`/`:nth-last-child` `an+b` micro-syntax parsing, isolated from
//! the main token stream (design §8): the content between `nth-child(` and
//! its matching `)` is extracted as a raw substring by
//! [`crate::css::parser`] and handed to [`parse_nth`] here, rather than
//! forcing the general CSS tokenizer to model the numeric `an+b` dimension
//! grammar (a well-known special case even in the CSS specification's own
//! tokenizer).

use crate::css::ast::NthExpr;
use crate::error::QueryError;

/// Parses the CSS `an+b` micro-syntax (`odd`, `even`, `3`, `-3`, `n`, `-n`,
/// `2n+1`, `-2n+1`, `n-1`, with optional internal whitespace around the `+`
/// or `-` before `b`) at `position` (used only for error reporting — the
/// caller supplies the byte offset of the raw content start).
pub(crate) fn parse_nth(raw: &str, position: usize) -> Result<NthExpr, QueryError> {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower == "odd" {
        return Ok(NthExpr { a: 2, b: 1 });
    }
    if lower == "even" {
        return Ok(NthExpr { a: 2, b: 0 });
    }
    let compact: String = lower.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.is_empty() {
        return Err(QueryError::InvalidSelector {
            message: "empty :nth-child(...) argument".to_string(),
            position,
        });
    }
    parse_compact_nth(&compact).ok_or_else(|| QueryError::InvalidSelector {
        message: format!("malformed an+b expression: '{trimmed}'"),
        position,
    })
}

fn parse_compact_nth(compact: &str) -> Option<NthExpr> {
    if let Some(n_index) = compact.find('n') {
        // Reject a second 'n' or any other stray letter — this must be a
        // pure an+b micro-syntax at this point ("odd"/"even" were already
        // handled by the caller).
        if compact[n_index + 1..].contains('n') || compact[..n_index].contains('n') {
            return None;
        }
        let a_part = &compact[..n_index];
        let a = match a_part {
            "" => 1,
            "+" => 1,
            "-" => -1,
            other => other.parse::<i32>().ok()?,
        };
        let rest = &compact[n_index + 1..];
        let b = if rest.is_empty() {
            0
        } else {
            rest.parse::<i32>().ok()?
        };
        Some(NthExpr { a, b })
    } else {
        let b = compact.parse::<i32>().ok()?;
        Some(NthExpr { a: 0, b })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nth(a: i32, b: i32) -> NthExpr {
        NthExpr { a, b }
    }

    #[test]
    fn parses_keywords_and_integers() {
        assert_eq!(parse_nth("odd", 0).unwrap(), nth(2, 1));
        assert_eq!(parse_nth("even", 0).unwrap(), nth(2, 0));
        assert_eq!(parse_nth("3", 0).unwrap(), nth(0, 3));
        assert_eq!(parse_nth("-3", 0).unwrap(), nth(0, -3));
    }

    #[test]
    fn parses_an_plus_b_forms() {
        assert_eq!(parse_nth("n", 0).unwrap(), nth(1, 0));
        assert_eq!(parse_nth("-n", 0).unwrap(), nth(-1, 0));
        assert_eq!(parse_nth("2n+1", 0).unwrap(), nth(2, 1));
        assert_eq!(parse_nth("2n + 1", 0).unwrap(), nth(2, 1));
        assert_eq!(parse_nth("-2n+1", 0).unwrap(), nth(-2, 1));
        assert_eq!(parse_nth("n-1", 0).unwrap(), nth(1, -1));
    }

    #[test]
    fn rejects_malformed_expressions() {
        assert!(parse_nth("", 0).is_err());
        assert!(parse_nth("nn", 0).is_err());
        assert!(parse_nth("2n+", 0).is_err());
        assert!(parse_nth("abc", 0).is_err());
    }
}
