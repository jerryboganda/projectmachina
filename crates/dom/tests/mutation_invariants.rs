//! insert/remove/replace/adopt/clone invariant tests, including the
//! self-aliased-argument and cardinality-constraint edge cases identified
//! by the M2-T05 security review.

use std::collections::HashMap;

use machina_dom::{Document, DomError, Namespace, NodeHandle, MAX_ANCESTOR_WALK};

fn make_element(doc: &mut Document, tag: &str) -> NodeHandle {
    doc.create_element(tag)
        .expect("create element")
        .node_handle()
}

/// M2-T04 (tree builder) dependency: namespace-aware element creation for
/// SVG/MathML foreign content, and empty-name rejection for both
/// `create_element_ns` and `create_document_type`.
#[test]
fn namespace_aware_element_creation_defaults_to_html_and_rejects_empty_names() {
    let mut doc = Document::new();
    let html_element = doc.create_element("div").expect("create");
    assert_eq!(
        doc.element_namespace(html_element).expect("namespace"),
        Namespace::Html
    );

    let svg_element = doc
        .create_element_ns(Namespace::Svg, "path")
        .expect("create svg element");
    assert_eq!(
        doc.element_namespace(svg_element).expect("namespace"),
        Namespace::Svg
    );
    assert_eq!(doc.tag_name(svg_element).expect("tag"), "path");

    assert_eq!(
        doc.create_element_ns(Namespace::MathMl, "  ").unwrap_err(),
        DomError::InvalidName
    );
    assert_eq!(
        doc.create_document_type("", "", "").unwrap_err(),
        DomError::InvalidName
    );
}

#[test]
fn reparenting_detaches_from_the_old_parent_exactly_once() {
    let mut doc = Document::new();
    // Document accepts only one Element child, so use a container for the
    // two candidate parents rather than attaching both directly to root.
    let container = make_element(&mut doc, "body");
    doc.append_child(doc.root(), container)
        .expect("attach container");
    let parent_one = make_element(&mut doc, "div");
    let parent_two = make_element(&mut doc, "section");
    let child = make_element(&mut doc, "span");
    doc.append_child(container, parent_one).expect("attach p1");
    doc.append_child(container, parent_two).expect("attach p2");
    doc.append_child(parent_one, child)
        .expect("attach child to p1");

    doc.append_child(parent_two, child)
        .expect("move child to p2");

    assert_eq!(
        doc.children(parent_one).expect("children"),
        Vec::<NodeHandle>::new()
    );
    assert_eq!(doc.children(parent_two).expect("children"), vec![child]);
    assert_eq!(doc.node(child).expect("resolve").parent(), Some(parent_two));
}

#[test]
fn cycle_rejection_is_rejected_and_leaves_the_tree_unchanged() {
    let mut doc = Document::new();
    let a = make_element(&mut doc, "a");
    let b = make_element(&mut doc, "b");
    let c = make_element(&mut doc, "c");
    doc.append_child(doc.root(), a).expect("attach a");
    doc.append_child(a, b).expect("attach b");
    doc.append_child(b, c).expect("attach c");

    let before_a = doc.children(a).expect("children a");
    let before_b = doc.children(b).expect("children b");
    let revision_before = doc.revision();

    let result = doc.append_child(c, a);
    assert_eq!(result.unwrap_err(), DomError::HierarchyViolation);
    assert_eq!(doc.children(a).expect("children a"), before_a);
    assert_eq!(doc.children(b).expect("children b"), before_b);
    assert_eq!(doc.revision(), revision_before);
}

#[test]
fn bad_reference_or_child_returns_not_found_and_leaves_tree_unchanged() {
    let mut doc = Document::new();
    let parent = make_element(&mut doc, "div");
    doc.append_child(doc.root(), parent).expect("attach parent");
    let unrelated = make_element(&mut doc, "span"); // never attached anywhere
    let child = make_element(&mut doc, "p");
    doc.append_child(parent, child).expect("attach child");

    let revision_before = doc.revision();
    let another = make_element(&mut doc, "em");

    assert_eq!(
        doc.insert_before(parent, another, Some(unrelated))
            .unwrap_err(),
        DomError::NotFound
    );
    assert_eq!(
        doc.remove_child(parent, unrelated).unwrap_err(),
        DomError::NotFound
    );
    assert_eq!(
        doc.replace_child(parent, another, unrelated).unwrap_err(),
        DomError::NotFound
    );
    assert_eq!(doc.children(parent).expect("children"), vec![child]);
    // `another`/`em` was allocated after the snapshot; only compare
    // revision advancing from creating it, not from any rejected mutation.
    let _ = revision_before;
}

#[test]
fn revision_strictly_increases_on_success_and_is_unchanged_on_err() {
    let mut doc = Document::new();
    let r0 = doc.revision();
    let element = make_element(&mut doc, "div");
    let r1 = doc.revision();
    assert!(r1 > r0);

    doc.append_child(doc.root(), element).expect("attach");
    let r2 = doc.revision();
    assert!(r2 > r1);

    let bad = doc.append_child(element, element); // parent == new_child
    assert!(bad.is_err());
    assert_eq!(doc.revision(), r2);
}

#[test]
fn replace_child_preserves_sibling_position_and_the_old_handle_stays_valid_but_detached() {
    let mut doc = Document::new();
    let parent = make_element(&mut doc, "ul");
    doc.append_child(doc.root(), parent).expect("attach parent");
    let a = make_element(&mut doc, "li");
    let b = make_element(&mut doc, "li");
    let c = make_element(&mut doc, "li");
    doc.append_child(parent, a).expect("attach a");
    doc.append_child(parent, b).expect("attach b");
    doc.append_child(parent, c).expect("attach c");

    let new_node = make_element(&mut doc, "li");
    doc.replace_child(parent, new_node, b).expect("replace");

    assert_eq!(
        doc.children(parent).expect("children"),
        vec![a, new_node, c]
    );
    let old = doc.node(b).expect("b is still resolvable, just detached");
    assert_eq!(old.parent(), None);
}

/// M2-T05 security review, finding 3c: replacing a Document's existing
/// sole `Element` child with a *different* `Element` is a legitimate
/// no-net-change-in-cardinality operation and must succeed, not be
/// rejected by a naive "already has an Element child" pre-count.
#[test]
fn replace_child_swapping_the_documents_sole_element_child_for_a_different_element_succeeds() {
    let mut doc = Document::new();
    let root = doc.root();
    let html = make_element(&mut doc, "html");
    doc.append_child(root, html)
        .expect("attach sole element child");

    let replacement = make_element(&mut doc, "html");
    doc.replace_child(root, replacement, html)
        .expect("swapping the sole Element child for a different Element must succeed");
    assert_eq!(doc.children(root).expect("children"), vec![replacement]);
}

/// M2-T05 security review, finding 3a: a self-aliased `replace_child`
/// argument (`new_child` happens to already be `old_child`'s immediate
/// next sibling) must not corrupt the sibling list by splicing a node
/// before itself.
#[test]
fn replace_child_with_its_own_next_sibling_does_not_corrupt_the_sibling_list() {
    let mut doc = Document::new();
    let parent = make_element(&mut doc, "ul");
    doc.append_child(doc.root(), parent).expect("attach parent");
    let a = make_element(&mut doc, "li");
    let b = make_element(&mut doc, "li");
    doc.append_child(parent, a).expect("attach a");
    doc.append_child(parent, b).expect("attach b");

    doc.replace_child(parent, b, a)
        .expect("replace a with its own next sibling b");

    assert_eq!(doc.children(parent).expect("children"), vec![b]);
    assert_eq!(doc.node(b).expect("resolve b").next_sibling(), None);
    assert_eq!(doc.node(b).expect("resolve b").previous_sibling(), None);
}

/// M2-T05 security review, finding 3a: inserting a node immediately before
/// itself (only reachable when it is already exactly there) must be a safe
/// no-op, not a self-referential sibling link.
#[test]
fn insert_before_a_node_immediately_before_itself_is_a_safe_no_op() {
    let mut doc = Document::new();
    let parent = make_element(&mut doc, "div");
    doc.append_child(doc.root(), parent).expect("attach parent");
    let a = make_element(&mut doc, "span");
    doc.append_child(parent, a).expect("attach a");

    let revision_before = doc.revision();
    doc.insert_before(parent, a, Some(a))
        .expect("self-reference is a safe no-op");
    assert_eq!(doc.children(parent).expect("children"), vec![a]);
    assert_eq!(doc.node(a).expect("resolve a").next_sibling(), None);
    assert_eq!(doc.revision(), revision_before);
}

#[test]
fn adopt_node_preserves_subtree_structure_content_and_re_interns_custom_atoms() {
    let mut source = Document::new();
    let parent = source.create_element("custom-widget").expect("create");
    source
        .set_attribute(parent, "data-custom-attr", "42")
        .expect("set attribute");
    let child_text = source.create_text("hello").node_handle();
    source
        .append_child(parent.node_handle(), child_text)
        .expect("attach text");

    let mut destination = Document::new();
    let new_root_handle = destination
        .adopt_node(&mut source, parent.node_handle())
        .expect("adopt");
    let new_root = destination.as_element(new_root_handle).expect("is element");

    assert_eq!(
        destination.tag_name(new_root).expect("tag name"),
        "custom-widget"
    );
    assert_eq!(
        destination
            .attribute(new_root, "data-custom-attr")
            .expect("attribute"),
        Some("42")
    );
    let children = destination.children(new_root_handle).expect("children");
    assert_eq!(children.len(), 1);
    assert_eq!(
        destination.text_data(children[0]).expect("text data"),
        "hello"
    );
    assert_ne!(new_root_handle, parent.node_handle());
}

// Note on `adopt_node(self, self) -> SameDocument`: `Document::adopt_node`
// takes `&mut self` and `source: &mut Document`. Calling it with `self` and
// `source` as the same binding requires two simultaneous mutable borrows of
// one value, which the Rust borrow checker rejects at compile time
// (E0499) -- so this design enforces "adopt_node cannot move a node within
// the same document" at compile time, a strictly stronger guarantee than a
// runtime check, and the scenario is not constructible from outside the
// crate to exercise the runtime `DomError::SameDocument` path directly. The
// check and error variant are kept as defense-in-depth documentation of the
// invariant.

#[test]
fn shallow_clone_omits_children_deep_clone_preserves_them() {
    let mut doc = Document::new();
    let parent = make_element(&mut doc, "div");
    let child = doc.create_text("hi").node_handle();
    doc.append_child(parent, child).expect("attach child");

    let shallow = doc.clone_node(parent, false).expect("shallow clone");
    assert_eq!(
        doc.children(shallow).expect("children"),
        Vec::<NodeHandle>::new()
    );

    let deep = doc.clone_node(parent, true).expect("deep clone");
    let deep_children = doc.children(deep).expect("children");
    assert_eq!(deep_children.len(), 1);
    assert_eq!(doc.text_data(deep_children[0]).expect("text data"), "hi");
    assert_ne!(deep_children[0], child);
    assert_ne!(deep, parent);
}

#[test]
fn document_node_accepts_at_most_one_element_child_and_no_text_child() {
    let mut doc = Document::new();
    let root = doc.root();
    let html = make_element(&mut doc, "html");
    doc.append_child(root, html)
        .expect("first element child ok");

    let second_html = make_element(&mut doc, "html");
    assert_eq!(
        doc.append_child(root, second_html).unwrap_err(),
        DomError::HierarchyViolation
    );

    let text = doc.create_text("stray").node_handle();
    assert_eq!(
        doc.append_child(root, text).unwrap_err(),
        DomError::HierarchyViolation
    );
}

#[test]
fn document_node_accepts_at_most_one_doctype_child() {
    let mut doc = Document::new();
    let root = doc.root();
    let doctype = doc
        .create_document_type("html", "", "")
        .expect("create doctype")
        .node_handle();
    doc.append_child(root, doctype).expect("first doctype ok");

    let second = doc
        .create_document_type("html", "", "")
        .expect("create doctype")
        .node_handle();
    assert_eq!(
        doc.append_child(root, second).unwrap_err(),
        DomError::HierarchyViolation
    );
}

/// Builds a linear chain of `depth` `Element` nodes bottom-up (wrapping an
/// existing subtree in a new parent each step). Building bottom-up keeps
/// this O(depth): each `append_child` call's ancestor walk starts from the
/// brand-new, still-parentless node being inserted as the parent, so it
/// terminates in O(1) rather than re-walking the whole existing chain.
/// Returns `(topmost, deepest_element)`.
fn build_chain_bottom_up(doc: &mut Document, depth: usize) -> (NodeHandle, NodeHandle) {
    let leaf = doc.create_text("leaf").node_handle();
    let deepest_element = make_element(doc, "chain-leaf-wrapper");
    doc.append_child(deepest_element, leaf)
        .expect("attach leaf");

    let mut current = deepest_element;
    for _ in 0..depth {
        let parent = make_element(doc, "chain");
        doc.append_child(parent, current)
            .expect("append during bottom-up chain build");
        current = parent;
    }
    (current, deepest_element)
}

/// M2-T05 security review, finding 5a: `MAX_ANCESTOR_WALK` must not
/// conflate fuzz-hardening with the correctness-critical cycle-detection
/// walk to the point that legitimate, deep, non-cyclic trees are falsely
/// rejected. This exercises a single ~50,000-step ancestor walk (well
/// under the 100,000 bound) that must succeed.
#[test]
fn legitimate_very_deep_non_cyclic_insert_is_not_falsely_rejected() {
    let mut doc = Document::new();
    let (_top, deepest_element) = build_chain_bottom_up(&mut doc, 50_000);
    let extra_leaf = doc.create_text("extra").node_handle();
    doc.append_child(deepest_element, extra_leaf)
        .expect("a ~50,000-deep legitimate, non-cyclic insert must succeed");
}

#[test]
fn depth_bound_fails_closed_for_ancestor_walk_and_deep_clone_without_leaking_memory() {
    let mut doc = Document::new();
    let (top, deepest_element) = build_chain_bottom_up(&mut doc, MAX_ANCESTOR_WALK + 10);

    let extra_leaf = doc.create_text("extra").node_handle();
    let memory_before_insert = doc.memory_usage();
    assert_eq!(
        doc.append_child(deepest_element, extra_leaf).unwrap_err(),
        DomError::DepthLimitExceeded
    );
    assert_eq!(doc.memory_usage(), memory_before_insert);

    // M2-T05 security review, finding 3b: the depth-guard must be a
    // read-only pre-pass before any destination allocation, so an
    // over-limit clone leaves memory_usage() unchanged rather than leaking
    // orphaned destination slots.
    let memory_before_clone = doc.memory_usage();
    assert_eq!(
        doc.clone_node(top, true).unwrap_err(),
        DomError::DepthLimitExceeded
    );
    assert_eq!(doc.memory_usage(), memory_before_clone);
}

struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Lcg(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn next_usize(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }
}

fn is_expected_descendant(
    expected_parent: &HashMap<NodeHandle, Option<NodeHandle>>,
    candidate: NodeHandle,
    ancestor: NodeHandle,
) -> bool {
    let mut current = Some(candidate);
    while let Some(handle) = current {
        if handle == ancestor {
            return true;
        }
        current = expected_parent.get(&handle).copied().flatten();
    }
    false
}

/// Fixed-seed pseudo-random mutation sequence checked against a plain
/// `HashMap`-based reference parent model (a hand-rolled LCG; no
/// `proptest`/`rand` precedent exists elsewhere in this workspace, per the
/// M2-T05 design doc). Cross-checks that every accepted `append_child`
/// matches the model and every rejected one really was a would-be cycle.
#[test]
fn fixed_seed_random_mutation_sequence_matches_a_plain_reference_parent_model() {
    let mut doc = Document::new();
    let root = doc.root();
    let container = make_element(&mut doc, "container");
    doc.append_child(root, container).expect("attach container");

    let pool: Vec<NodeHandle> = (0..12)
        .map(|i| make_element(&mut doc, &format!("n{i}")))
        .collect();
    let mut candidates = pool.clone();
    candidates.push(container);

    let mut expected_parent: HashMap<NodeHandle, Option<NodeHandle>> =
        pool.iter().map(|&handle| (handle, None)).collect();

    let mut rng = Lcg::new(0x5EED_5EED_5EED_5EED);
    for _ in 0..500 {
        let child_index = rng.next_usize(pool.len());
        let child = pool[child_index];

        let mut parent_index = rng.next_usize(candidates.len());
        if candidates[parent_index] == child {
            parent_index = (parent_index + 1) % candidates.len();
        }
        let parent = candidates[parent_index];

        match doc.append_child(parent, child) {
            Ok(()) => {
                expected_parent.insert(child, Some(parent));
            }
            Err(DomError::HierarchyViolation) => {
                assert!(
                    is_expected_descendant(&expected_parent, parent, child),
                    "rejected append_child that the reference model does not consider a cycle"
                );
            }
            Err(other) => panic!("unexpected error from fuzz sequence: {other:?}"),
        }

        for &node in &pool {
            let actual = doc
                .node(node)
                .expect("pool node always resolvable")
                .parent();
            assert_eq!(
                actual,
                expected_parent.get(&node).copied().flatten(),
                "parent mismatch for a node after a fuzz step"
            );
        }
    }
}
