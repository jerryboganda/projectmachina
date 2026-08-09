//! The adoption agency algorithm (WHATWG HTML §13.2.6.4.7, design §3):
//! recovers from misnested formatting elements (`<b><i>x</b>y</i>`-style
//! input) deterministically. Bookmark/index bookkeeping uses handle-based
//! re-resolution at every step (never raw indices carried across a
//! mutation) specifically so a stack mutation mid-algorithm can never
//! desync into an out-of-bounds access — safety is structural here, not
//! just tested behavior. Loop bounds are the spec's own fixed, small
//! constants (outer <= 8, inner <= 3), enforced as literal counter checks
//! (design §3, §7d).
//!
//! Honest scope note (see `.agent-state/evidence/M2-T04.md`): this is a
//! faithful structural implementation of the algorithm's steps, verified
//! against the common single-mismatch case (a formatting element closed
//! while exactly one special "furthest block" descendant is still open).
//! It has **not** been validated against the full html5lib-tests
//! `adoption01.dat`/`adoption02.dat` corpus (that corpus does not exist in
//! this repository yet — no vendoring/WPT harness has been built by any
//! task through M2-T04 — deferred to real WPT infrastructure work).

use machina_dom::{Document, ElementHandle, Namespace};

use crate::active_formatting::FormattingEntry;
use crate::builder::TreeBuilder;
use crate::error::TreeBuilderError;
use crate::open_elements::OpenElementEntry;

const OUTER_LOOP_MAX: usize = 8;
const INNER_LOOP_MAX: usize = 3;

pub(crate) fn run_adoption_agency(
    builder: &mut TreeBuilder,
    doc: &mut Document,
    subject: &str,
) -> Result<(), TreeBuilderError> {
    // Step 2: current node is subject and not in AFE -> pop, done.
    if let Some(current) = builder.open_elements.current() {
        if current.namespace == Namespace::Html
            && current.tag == subject
            && builder.afe.position_of(current.handle).is_none()
        {
            builder.open_elements.pop();
            return Ok(());
        }
    }

    let mut outer_loop = 0usize;
    loop {
        if outer_loop >= OUTER_LOOP_MAX {
            return Ok(());
        }
        outer_loop += 1;

        let Some((_afe_index, fe_handle, fe_attrs, fe_namespace)) =
            builder.afe.last_between_end_and_marker_with_tag(subject)
        else {
            builder.any_other_end_tag_in_body(subject);
            return Ok(());
        };

        let Some(fe_stack_index) = builder.open_elements.index_of_handle(fe_handle) else {
            builder.record_parse_error(format!(
                "adoption agency: <{subject}> formatting element is not open"
            ));
            if let Some(pos) = builder.afe.position_of(fe_handle) {
                builder.afe.remove_at(pos);
            }
            return Ok(());
        };

        if !builder.open_elements.has_in_scope(subject) {
            builder.record_parse_error(format!("adoption agency: <{subject}> is not in scope"));
            return Ok(());
        }

        if builder.open_elements.current_handle() != Some(fe_handle) {
            builder.record_parse_error(format!(
                "adoption agency: <{subject}> is not the current node"
            ));
        }

        // Step 7: furthest block = first "special" element found scanning
        // upward (toward the top of the stack) from immediately above the
        // formatting element.
        let mut furthest_block_index = None;
        for i in (fe_stack_index + 1)..builder.open_elements.len() {
            let Some(entry) = builder.open_elements.entry_at(i) else {
                break;
            };
            if crate::special::is_special(&entry.tag, entry.namespace) {
                furthest_block_index = Some(i);
                break;
            }
        }

        let Some(furthest_block_index) = furthest_block_index else {
            // Step 8: no furthest block — pop up to and including the
            // formatting element, drop it from the AFE list, done.
            builder.open_elements.pop_until_handle(fe_handle);
            if let Some(pos) = builder.afe.position_of(fe_handle) {
                builder.afe.remove_at(pos);
            }
            return Ok(());
        };

        if fe_stack_index == 0 {
            // Defensive: a real formatting element is never the bottommost
            // (<html>) stack entry, but fail closed rather than underflow.
            return Ok(());
        }
        let Some(common_ancestor_handle) = builder
            .open_elements
            .entry_at(fe_stack_index - 1)
            .map(|e| e.handle)
        else {
            return Ok(());
        };
        let Some(furthest_block_handle) = builder
            .open_elements
            .entry_at(furthest_block_index)
            .map(|e| e.handle)
        else {
            return Ok(());
        };

        let mut bookmark = builder.afe.position_of(fe_handle).unwrap_or(0);

        let mut node_handle: ElementHandle = furthest_block_handle;
        let mut last_node_handle: ElementHandle = furthest_block_handle;
        let mut inner_loop = 0usize;
        loop {
            inner_loop += 1;
            let Some(node_index) = builder.open_elements.index_of_handle(node_handle) else {
                break;
            };
            if node_index == 0 {
                break;
            }
            let above_index = node_index - 1;
            let Some(above_handle) = builder
                .open_elements
                .entry_at(above_index)
                .map(|e| e.handle)
            else {
                break;
            };
            node_handle = above_handle;
            if node_handle == fe_handle {
                break;
            }

            let node_afe_pos = builder.afe.position_of(node_handle);

            if inner_loop > INNER_LOOP_MAX {
                if let Some(pos) = node_afe_pos {
                    builder.afe.remove_at(pos);
                    if pos < bookmark {
                        bookmark = bookmark.saturating_sub(1);
                    }
                }
                if let Some(idx) = builder.open_elements.index_of_handle(node_handle) {
                    builder.open_elements.remove_at(idx);
                }
                continue;
            }

            let Some(node_afe_pos) = node_afe_pos else {
                if let Some(idx) = builder.open_elements.index_of_handle(node_handle) {
                    builder.open_elements.remove_at(idx);
                }
                continue;
            };

            let (tag, namespace, attrs) = match builder.afe.entry_at(node_afe_pos) {
                Some(FormattingEntry::Element {
                    tag,
                    namespace,
                    attrs,
                    ..
                }) => (tag.clone(), *namespace, attrs.clone()),
                _ => break,
            };

            let new_handle = builder.create_detached_element(doc, &tag, namespace, &attrs)?;
            if let Some(idx) = builder.open_elements.index_of_handle(node_handle) {
                builder.open_elements.replace_at(
                    idx,
                    OpenElementEntry {
                        handle: new_handle,
                        tag: tag.clone(),
                        namespace,
                    },
                );
            }
            builder.afe.replace_at(
                node_afe_pos,
                FormattingEntry::Element {
                    handle: new_handle,
                    tag: tag.clone(),
                    namespace,
                    attrs,
                },
            );

            if last_node_handle == furthest_block_handle {
                bookmark = node_afe_pos + 1;
            }

            builder.append_child_checked(
                doc,
                new_handle.node_handle(),
                last_node_handle.node_handle(),
            )?;

            last_node_handle = new_handle;
            node_handle = new_handle;
        }

        // Step 14: place lastNode under commonAncestor (foster-parenting
        // if commonAncestor is table-context).
        builder.place_under_common_ancestor(doc, common_ancestor_handle, last_node_handle)?;

        // Steps 15-17: a fresh clone of the formatting element's token
        // adopts all of furthestBlock's current children, then becomes
        // furthestBlock's child; bookkeeping in both the AFE list and the
        // open-elements stack is updated to reflect the new element in
        // place of the old one.
        let new_fe_handle =
            builder.create_detached_element(doc, subject, fe_namespace, &fe_attrs)?;
        builder.move_all_children(
            doc,
            furthest_block_handle.node_handle(),
            new_fe_handle.node_handle(),
        )?;
        builder.append_child_checked(
            doc,
            furthest_block_handle.node_handle(),
            new_fe_handle.node_handle(),
        )?;

        if let Some(pos) = builder.afe.position_of(fe_handle) {
            builder.afe.remove_at(pos);
            if pos < bookmark {
                bookmark = bookmark.saturating_sub(1);
            }
        }
        let bookmark = bookmark.min(builder.afe.len());
        builder.afe.insert_at(
            bookmark,
            FormattingEntry::Element {
                handle: new_fe_handle,
                tag: subject.to_string(),
                namespace: fe_namespace,
                attrs: fe_attrs.clone(),
            },
        );

        if let Some(idx) = builder.open_elements.index_of_handle(fe_handle) {
            builder.open_elements.remove_at(idx);
        }
        if let Some(idx) = builder.open_elements.index_of_handle(furthest_block_handle) {
            builder.open_elements.insert_at(
                idx + 1,
                OpenElementEntry {
                    handle: new_fe_handle,
                    tag: subject.to_string(),
                    namespace: fe_namespace,
                },
            );
        }
    }
}
