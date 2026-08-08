---
title: "Semantic Interaction Kernel"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define the machine-oriented representation of visibility, roles, names, interactability, geometry, and deltas."
---

# Semantic Interaction Kernel

## Purpose

AI agents and automation need more than raw DOM but often do not need pixels. The semantic kernel derives a stable, compact representation suitable for element discovery, verified actions, extraction, and change deltas.

## Inputs

- DOM and composed tree;
- selected CSS cascade/layout properties;
- ARIA attributes and accessibility-name rules;
- form, focus, disabled, inert, hidden, checked, selected, expanded, required, and validation state;
- viewport and simplified geometry;
- event/action state;
- origin/frame boundaries and policy.

## Outputs

### Semantic node

```json
{
  "semantic_id": "s-...",
  "dom_handle": "n-...",
  "role": "button",
  "name": "Submit order",
  "description": null,
  "states": ["enabled", "focusable"],
  "visible": true,
  "interactable": true,
  "frame": "f-...",
  "order": 42,
  "bounds": {"x": 10, "y": 20, "width": 120, "height": 32},
  "revision": 184
}
```

### Derived views

- full semantic tree;
- interactive-element index;
- headings/landmarks/forms/links tables;
- readable markdown;
- structured metadata and schema outputs;
- delta since revision.

## Visibility/interactability

At minimum consider:

- connection to active document/composed tree;
- `display`, `visibility`, opacity threshold policy, hidden/inert attributes;
- disabled state and ancestor controls;
- nonzero/sensible geometry where geometry exists;
- clipping/overflow and viewport intersection policy;
- obscuration/hit target when computed;
- pointer-events and focus behavior;
- stable position during the action window.

The native kernel reports confidence/limitations for geometry it does not fully compute. Visual-certainty requirements route to Chromium.

## Stable semantic IDs

IDs are session/document scoped and survive non-structural mutations where possible. Locators should store role/name/state plus structural hints rather than a fragile numeric ID alone. Stale IDs return revision information.

## Deltas

`getSemanticDelta(since_revision)` returns inserts, deletes, changes, reorders, and action-relevant state changes. It is bounded; clients receive `full_snapshot_required` if the revision window has expired.

## Single-pass derivation

One incremental traversal should update semantic indexes and feed markdown, form, links, headings, and schema extraction when requested. Avoid separate complete DOM walks for each representation.

## Privacy

Semantic output may contain page-sensitive text. Apply the same tenant, retention, redaction, and artifact policies as DOM content. Telemetry stores counts/timings by default, not node text.
