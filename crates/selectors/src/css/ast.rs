//! Immutable, `Document`-agnostic CSS selector AST. A compiled
//! [`SelectorList`] carries no reference to any particular document or
//! revision, so it is safely cacheable by a caller across documents and
//! time (design §4a).

/// A full selector list: `a, b, c` — matches if any member complex selector
/// matches (comma = logical OR).
#[derive(Clone, Debug, PartialEq)]
pub struct SelectorList {
    pub selectors: Vec<ComplexSelector>,
}

/// One comma-separated member: a sequence of compound selectors joined by
/// combinators, for example `div.card > p`. Stored in natural left-to-right
/// (source) order; the matcher (design §2) walks it right-to-left by
/// indexing from `compounds.len() - 1` down to `0`, which is algorithmically
/// equivalent to the design's "reversed sequence" representation without a
/// separate reversal pass at parse time.
#[derive(Clone, Debug, PartialEq)]
pub struct ComplexSelector {
    /// Compound selectors in source (left-to-right) order.
    pub compounds: Vec<CompoundSelector>,
    /// `combinators[i]` connects `compounds[i]` (left) to `compounds[i + 1]`
    /// (right). Always `compounds.len() - 1` entries.
    pub combinators: Vec<Combinator>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Combinator {
    /// Whitespace: `a b`.
    Descendant,
    /// `a > b`.
    Child,
    /// `a + b`.
    AdjacentSibling,
    /// `a ~ b`.
    GeneralSibling,
}

/// One compound selector: a run of simple selectors with no combinator
/// between them, for example `div.card#main[data-x]`. Sorted cheapest-first
/// at parse time (design §2's compound-match order) so a mismatch on a
/// cheap simple selector never pays for an expensive structural
/// pseudo-class check.
#[derive(Clone, Debug, PartialEq)]
pub struct CompoundSelector {
    pub simple_selectors: Vec<SimpleSelector>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SimpleSelector {
    Universal,
    /// Lowercased element (tag) name.
    Type(String),
    Class(String),
    Id(String),
    Attribute {
        /// Attribute name, as written (attribute name matching against the
        /// DOM is exact-string via `machina_dom::Document::attribute`, not
        /// case-folded — see the design doc §1's case-sensitivity note: tag
        /// names are folded for free by the tree builder, attribute *names*
        /// on real HTML documents are already lowercase by the time they
        /// reach this crate).
        name: String,
        operator: AttrOperator,
    },
    PseudoClass(PseudoClass),
    /// `:not(<compound>)` — negates a single compound selector (design §1:
    /// deliberately not a full selector list inside `:not()`).
    Negation(Box<CompoundSelector>),
}

impl SimpleSelector {
    /// Cheapest-first match-order rank (design §2): id/type first (single
    /// atom/string comparison), then class/attribute-presence (linear
    /// attribute scan), then attribute-value operators (string compare),
    /// then structural pseudo-classes last (sibling-position counting, the
    /// most expensive check).
    pub(crate) fn match_cost_rank(&self) -> u8 {
        match self {
            SimpleSelector::Id(_) | SimpleSelector::Type(_) => 0,
            SimpleSelector::Universal => 0,
            SimpleSelector::Class(_) => 1,
            SimpleSelector::Attribute {
                operator: AttrOperator::Present,
                ..
            } => 1,
            SimpleSelector::Attribute { .. } => 2,
            SimpleSelector::Negation(_) => 2,
            SimpleSelector::PseudoClass(_) => 3,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AttrOperator {
    /// `[attr]`
    Present,
    /// `[attr=val]`
    Equals(String),
    /// `[attr~=val]` — value is a whitespace-separated word list containing
    /// exactly `val`.
    Includes(String),
    /// `[attr|=val]` — value equals `val` or starts with `val` followed by
    /// `-`.
    DashMatch(String),
    /// `[attr^=val]`
    PrefixMatch(String),
    /// `[attr$=val]`
    SuffixMatch(String),
    /// `[attr*=val]`
    SubstringMatch(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum PseudoClass {
    FirstChild,
    LastChild,
    OnlyChild,
    Empty,
    Root,
    NthChild(NthExpr),
    NthLastChild(NthExpr),
}

/// The CSS `an+b` micro-syntax: matches 1-based sibling position `p` when
/// `p = a*n + b` for some integer `n >= 0`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NthExpr {
    pub a: i32,
    pub b: i32,
}

impl NthExpr {
    pub fn matches(&self, position: i64) -> bool {
        let b = self.b as i64;
        let a = self.a as i64;
        if a == 0 {
            return position == b;
        }
        let diff = position - b;
        if diff % a != 0 {
            return false;
        }
        diff / a >= 0
    }
}
