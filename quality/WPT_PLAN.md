---
title: "Web Platform Tests Plan"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Use WPT systematically to drive prioritized web standards conformance without blocking useful hybrid delivery."
---

# Web Platform Tests Plan

## Strategy

Vendor a pinned WPT revision or fetch it reproducibly. Run tests through the native engine using WebDriver BiDi or a dedicated standards harness that preserves WPT semantics. Use Chromium/reference results for comparison, not as unquestioned truth where standards differ.

## Priority order

### P0

- URL, encoding, MIME.
- HTML parsing and DOM.
- events, focus, forms, input.
- navigation, history, browsing contexts.
- fetch, CORS, redirects, cookies.
- web storage.
- Shadow DOM and custom elements.

### P1

- frames and cross-origin behavior.
- workers, WebSockets.
- service workers and IndexedDB.
- selected CSSOM/visibility semantics.
- streams, abort, structured clone, messaging.

### Fallback-only

Visual reftests, paint/font/layout fidelity, canvas/GPU/media where native implementation is not claimed.

## Metadata

Track expected results by exact engine build and WPT revision. Every expected failure links to capability status/issue and has owner/review date. Do not blanket-skip directories to inflate percentages.

## Sharding

Shard by subsystem and historical duration. Tier 1 runs only touched focused tests; Tier 2 runs small representative shards; M8/M9 runs full prioritized sets. Record retries separately and report first-attempt stability.

## Harness requirements

- deterministic local HTTPS/HTTP hosts and certificates;
- required origins/subdomains/ports;
- timeouts appropriate to engine but not inflated without evidence;
- crash, stderr, console, and trace capture;
- machine-readable results and artifact links;
- support for filtered reproduction command.

## Reporting

Report pass/fail/timeout/crash/not-run/expected status, regression/new-pass, flaky evidence, and native capability implication. Overall percentage is secondary to priority-subsystem readiness and absence of severe semantic gaps.
