//! Named character reference (HTML entity) table — deliberately reduced MVP
//! subset, not the full WHATWG ~2231-entry table. See
//! `data/PROVENANCE.md` and `.agent-state/evidence/M2-T03.md` for exactly
//! what was vendored, from where, and what remains a tracked deferred item.
//!
//! Each entry is `(name, codepoint, legacy)` where `name` is the entity
//! identifier **without** the leading `&` or trailing `;` (e.g. `"amp"`,
//! `"AMP"`), `codepoint` is its single-scalar-value replacement (every entry
//! in this reduced subset happens to decode to exactly one code point — the
//! WHATWG table's small number of two-code-point legacy entities, e.g.
//! `&NotEqualTilde;`, are out of scope for this MVP subset), and `legacy`
//! records whether the historical no-trailing-semicolon form
//! (`&name` with no `;`) is also valid per WHATWG HTML §13.5's legacy list.
//!
//! Sourced from the real, currently-published WHATWG table
//! (`https://html.spec.whatwg.org/entities.json`, see `data/PROVENANCE.md`
//! for the exact fetch date/commit/license) — the *values* are real spec
//! data, only the *set* of entries is a hand-curated, high-frequency subset.
//!
//! Sorted by `name` (byte-lexicographic) so the matcher below can do
//! straightforward prefix scanning; a DAFSA/perfect-hash compression or
//! true binary search is a non-required fast-follow once the table grows
//! (see design §2).
pub(crate) static ENTITY_TABLE: &[(&str, u32, bool)] = &[
    ("AMP", 0x26, true),
    ("Auml", 0xC4, true),
    ("COPY", 0xA9, true),
    ("Dagger", 0x2021, false),
    ("GT", 0x3E, true),
    ("LT", 0x3C, true),
    ("Ouml", 0xD6, true),
    ("QUOT", 0x22, true),
    ("REG", 0xAE, true),
    ("Uuml", 0xDC, true),
    ("agrave", 0xE0, true),
    ("amp", 0x26, true),
    ("apos", 0x27, false),
    ("auml", 0xE4, true),
    ("bull", 0x2022, false),
    ("ccedil", 0xE7, true),
    ("cent", 0xA2, true),
    ("copy", 0xA9, true),
    ("dagger", 0x2020, false),
    ("darr", 0x2193, false),
    ("deg", 0xB0, true),
    ("divide", 0xF7, true),
    ("eacute", 0xE9, true),
    ("egrave", 0xE8, true),
    ("euro", 0x20AC, false),
    ("frac12", 0xBD, true),
    ("frac14", 0xBC, true),
    ("frac34", 0xBE, true),
    ("ge", 0x2265, false),
    ("gt", 0x3E, true),
    ("harr", 0x2194, false),
    ("hellip", 0x2026, false),
    ("infin", 0x221E, false),
    ("laquo", 0xAB, true),
    ("larr", 0x2190, false),
    ("ldquo", 0x201C, false),
    ("le", 0x2264, false),
    ("lsquo", 0x2018, false),
    ("lt", 0x3C, true),
    ("mdash", 0x2014, false),
    ("middot", 0xB7, true),
    ("nbsp", 0xA0, true),
    ("ndash", 0x2013, false),
    ("ne", 0x2260, false),
    ("ntilde", 0xF1, true),
    ("ouml", 0xF6, true),
    ("para", 0xB6, true),
    ("permil", 0x2030, false),
    ("plusmn", 0xB1, true),
    ("pound", 0xA3, true),
    ("quot", 0x22, true),
    ("raquo", 0xBB, true),
    ("rarr", 0x2192, false),
    ("rdquo", 0x201D, false),
    ("reg", 0xAE, true),
    ("rsquo", 0x2019, false),
    ("sect", 0xA7, true),
    ("szlig", 0xDF, true),
    ("times", 0xD7, true),
    ("trade", 0x2122, false),
    ("uarr", 0x2191, false),
    ("uuml", 0xFC, true),
    ("yen", 0xA5, true),
];

/// True if some entry's name starts with `prefix`. Used to decide whether
/// scanning another character could still extend toward a match.
pub(crate) fn has_prefix(table: &[(&str, u32, bool)], prefix: &str) -> bool {
    table.iter().any(|(name, _, _)| name.starts_with(prefix))
}

/// Exact-name lookup: returns `(codepoint, legacy_no_semicolon_ok)`.
pub(crate) fn exact_match(table: &[(&str, u32, bool)], name: &str) -> Option<(u32, bool)> {
    table
        .iter()
        .find(|(candidate, _, _)| *candidate == name)
        .map(|(_, codepoint, legacy)| (*codepoint, *legacy))
}

/// Numeric character reference remapping for the C1 control range
/// (`0x80..=0x9F`), per WHATWG HTML §13.2.5.80's fixed 32-entry table
/// (values without an explicit mapping fall through to the caller's default
/// "use the raw code point, flag it as a control-character-reference"
/// handling — see `src/tokenizer.rs`).
pub(crate) fn c1_control_replacement(code: u32) -> Option<u32> {
    let replacement = match code {
        0x80 => 0x20AC,
        0x82 => 0x201A,
        0x83 => 0x0192,
        0x84 => 0x201E,
        0x85 => 0x2026,
        0x86 => 0x2020,
        0x87 => 0x2021,
        0x88 => 0x02C6,
        0x89 => 0x2030,
        0x8A => 0x0160,
        0x8B => 0x2039,
        0x8C => 0x0152,
        0x8E => 0x017D,
        0x91 => 0x2018,
        0x92 => 0x2019,
        0x93 => 0x201C,
        0x94 => 0x201D,
        0x95 => 0x2022,
        0x96 => 0x2013,
        0x97 => 0x2014,
        0x98 => 0x02DC,
        0x99 => 0x2122,
        0x9A => 0x0161,
        0x9B => 0x203A,
        0x9C => 0x0153,
        0x9E => 0x017E,
        0x9F => 0x0178,
        _ => return None,
    };
    Some(replacement)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_sorted_and_has_no_duplicate_names() {
        for pair in ENTITY_TABLE.windows(2) {
            assert!(
                pair[0].0 < pair[1].0,
                "table must be strictly sorted: {:?} >= {:?}",
                pair[0].0,
                pair[1].0
            );
        }
    }

    #[test]
    fn known_entries_resolve() {
        assert_eq!(exact_match(ENTITY_TABLE, "amp"), Some((0x26, true)));
        assert_eq!(exact_match(ENTITY_TABLE, "apos"), Some((0x27, false)));
        assert_eq!(exact_match(ENTITY_TABLE, "notarealentity"), None);
    }

    #[test]
    fn prefix_matching_supports_a_synthetic_nested_table() {
        // The shipped ENTITY_TABLE has no entry that is a strict prefix of
        // another (verified structurally below), so this synthetic table
        // exercises the longest-prefix-match contract (design §2) directly,
        // independent of which names happen to be curated.
        let nested: &[(&str, u32, bool)] = &[("not", 0x1, false), ("notin", 0x2, false)];
        assert!(has_prefix(nested, "not"));
        assert!(has_prefix(nested, "noti"));
        assert!(!has_prefix(nested, "nope"));
        assert_eq!(exact_match(nested, "not"), Some((0x1, false)));
        assert_eq!(exact_match(nested, "notin"), Some((0x2, false)));
    }

    #[test]
    fn shipped_table_has_no_nested_prefix_pairs() {
        // Documents that the multi-character-pushback path in the tokenizer
        // (needed when a shorter complete entity name is itself a prefix of
        // a longer one) is exercised only by the synthetic-table test above,
        // not by any fixture using the shipped MVP table — an honest
        // statement of current coverage, not a design limitation.
        for &(name, _, _) in ENTITY_TABLE {
            for &(other, _, _) in ENTITY_TABLE {
                if name != other {
                    assert!(
                        !other.starts_with(name),
                        "{name:?} is a strict prefix of {other:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn c1_control_table_covers_documented_entries_and_leaves_gaps_undefined() {
        assert_eq!(c1_control_replacement(0x80), Some(0x20AC));
        assert_eq!(c1_control_replacement(0x9F), Some(0x0178));
        assert_eq!(c1_control_replacement(0x81), None);
    }
}
