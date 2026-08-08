---
title: "HTML, DOM, Events, and Interaction Model"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define parsing, node representation, mutation, selectors, events, focus, forms, and action semantics."
---

# HTML, DOM, Events, and Interaction Model

## HTML parsing

Implement a streaming tokenizer and tree builder aligned with HTML parsing algorithms, including error recovery, insertion modes, templates, foreign content, character references, scripts, and parser-blocking behavior required by target pages. Network chunks feed the parser under bounded buffering.

## DOM representation

Nodes use stable generational IDs within a document. Core node types include document, doctype, element, text, comment, document fragment, and shadow root. Attributes and namespaces are interned where beneficial. Mutation operations update tree, indexes, custom-element reactions, observers, semantic revision, and JavaScript wrappers atomically from the caller's perspective.

## Querying

- `getElementById`, tag/name/class collections where required.
- CSS selector matching/query with standards-tested parsing.
- XPath subset progressing to required compatibility.
- Semantic locators by role, accessible name, state, text, label, placeholder, and test ID.
- Stable locator resolution returns ambiguity and detachment errors explicitly.

## Events

Support event targets, capture/target/bubble phases, composed paths, cancellation, passive/once listeners, trusted versus synthetic distinction, default actions, and Shadow DOM retargeting. Event dispatch cannot mutate listener iteration unsafely.

## Focus and input

Maintain active element, focus/blur order, tab sequence, disabled/inert/hidden behavior, keyboard composition where prioritized, pointer/mouse synthesis, selection, and form control state.

## Forms

Implement text controls, checkbox/radio, select/options, buttons, labels, fieldsets, validation subset, form association, submission encoding, and navigation/fetch effects. File inputs use controlled platform handles and policy checks; page scripts never receive arbitrary host paths.

## Custom elements and Shadow DOM

Track custom-element definitions and reaction queues. Support open/closed shadow roots according to DOM access rules, slots/composed tree, event retargeting, and semantic traversal. Automation access to closed roots follows explicit product capability and does not pretend to be ordinary page JavaScript access.

## Mutation and semantic revisions

Each observable change increments scoped revisions. Derived semantic indexes are invalidated incrementally. Command outcomes return the revisions used for locator resolution and postcondition verification.

## Action contract

A click/fill/press/select/check command performs:

1. resolve locator at revision;
2. verify attached, visible, enabled, stable, and policy-allowed state;
3. optionally scroll/geometry prepare in the semantic kernel;
4. dispatch input/default behavior;
5. observe navigation/mutation/action effects;
6. verify declared postcondition;
7. return before/after revisions and event summary.

No action reports success solely because an event was dispatched.
