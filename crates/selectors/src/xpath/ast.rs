//! Immutable XPath AST for the grammar subset in scope (design §5):
//! absolute/relative location paths, `//` abbreviation, the five listed
//! axes, and simple predicates.

#[derive(Clone, Debug, PartialEq)]
pub struct XPathExpr {
    /// `true` for a path starting with `/` or `//`; `false` for a relative
    /// path, which requires a context node (design §5,
    /// `QueryError::ContextNodeRequired`).
    pub absolute: bool,
    pub steps: Vec<Step>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub axis: Axis,
    pub test: NodeTest,
    pub predicates: Vec<Predicate>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Axis {
    Child,
    Descendant,
    Attribute,
    SelfAxis,
    Parent,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NodeTest {
    /// A specific element name (`div`) or, on the attribute axis, a
    /// specific attribute name (`@id`).
    Name(String),
    /// `*` — any element (design §5: principal node type of the axis; on
    /// the attribute axis, `@*` is out of scope, see `xpath/parser.rs`).
    Any,
    Text,
    Node,
    Comment,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Predicate {
    /// `[N]` — 1-based position within the current step's per-context
    /// candidate list.
    Index(usize),
    /// `[last()]`
    Last,
    /// `[@attr]`
    AttributeExists(String),
    /// `[@attr='value']`
    AttributeEquals(String, String),
    /// `[P1 and P2]`
    And(Box<Predicate>, Box<Predicate>),
}
