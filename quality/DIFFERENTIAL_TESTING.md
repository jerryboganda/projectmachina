---
title: "Native and Reference Browser Differential Testing"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Compare observable task semantics across native and reference engines and turn differences into actionable capability evidence."
---

# Differential Testing

## Purpose

WPT covers standards but real automation combines APIs and timing. Differential tests execute the same deterministic workload in native and pinned Chromium, then compare normalized observable outcomes.

## Compared observations

- final URL, navigation/lifecycle sequence;
- selected DOM structure/attributes/text normalized by test;
- semantic roles/names/states/order;
- action success and postconditions;
- cookies and allowed storage fingerprints;
- request method/URL category/status/redirects/headers selected by fixture;
- console errors and JavaScript return values;
- downloads/dialogs/frame/worker events where applicable;
- error classification for deliberately invalid behavior.

## Non-comparable observations

Wall-clock ordering within tolerances, browser-specific stack formatting, random IDs, cache timing, visual geometry when native does not claim it, and known protocol representation differences. Normalize explicitly; never discard differences ad hoc.

## Corpus

1. Deterministic local fixtures by capability.
2. Recorded/controlled network fixtures for complex applications where legally permitted.
3. Approved real-site task corpus with stable postconditions and privacy-safe capture.
4. Historical regression cases from production and bugs.

## Oracle

The oracle is the declared requirement/standard plus normalized reference behavior. Chromium disagreement is investigated; it is not automatically proof native is wrong.

## Triage

Classify as native defect, reference/browser difference, test/normalizer defect, unsupported capability, nondeterminism, external site change, or policy difference. Update capability routing only after classification.

## Metrics

Semantic match rate, verified task success, divergence by capability, timeout/crash, false migration, missed migration, and regression/new-pass trend.
