//! Bounded-input limits (design §7).
//!
//! The tree builder enforces its own strictly-tighter depth ceiling
//! *before* any DOM call, so `machina_dom::DomError::HierarchyViolation`
//! (which fires past `machina_dom::MAX_ANCESTOR_WALK`) is provably
//! unreachable on any path the tree builder takes — not just "shouldn't
//! happen" abstractly. [`limits_invariant_holds`] is exercised as a unit
//! test in this module and additionally asserted once at every
//! [`TreeBuilderLimits`] construction site used by tests, so a future
//! accidental widening of `max_open_elements_depth` past
//! `machina_dom::MAX_ANCESTOR_WALK` fails fast in CI rather than silently
//! reintroducing the reachability the design explicitly closes off.

#[derive(Debug, Clone, Copy)]
pub struct TreeBuilderLimits {
    /// Hard ceiling on the stack of open elements. Adversarial unclosed
    /// same-tag nesting (design §7b) fails closed at this ceiling with
    /// [`crate::diagnostics::Diagnostic::NestingLimitExceeded`] before the
    /// offending element is ever created in the DOM.
    pub max_open_elements_depth: usize,
    /// Bound on "reprocess the token under a different insertion mode"
    /// hops per token (design §1, §7f) — the dispatch loop is iterative,
    /// not recursive, so this bounds total work per token rather than call
    /// stack depth.
    pub max_reprocess_hops: usize,
    /// Bound on the active formatting elements list, defense-in-depth
    /// against pathological formatting-element churn (the Noah's Ark
    /// clause already keeps same-tag/same-attrs runs to 3, but an attacker
    /// varying attributes on every open tag could otherwise grow this
    /// list unboundedly).
    pub max_active_formatting_elements: usize,
}

impl Default for TreeBuilderLimits {
    fn default() -> Self {
        let limits = Self {
            max_open_elements_depth: 4_000,
            max_reprocess_hops: 8,
            max_active_formatting_elements: 10_000,
        };
        debug_assert!(limits_invariant_holds(&limits));
        limits
    }
}

/// `true` iff `limits.max_open_elements_depth` is strictly less than
/// `machina_dom::MAX_ANCESTOR_WALK`, the invariant design §7a requires.
pub fn limits_invariant_holds(limits: &TreeBuilderLimits) -> bool {
    limits.max_open_elements_depth < machina_dom::MAX_ANCESTOR_WALK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_limits_satisfy_the_dom_hierarchy_violation_unreachability_invariant() {
        assert!(limits_invariant_holds(&TreeBuilderLimits::default()));
    }
}
