---
title: "WebDriver BiDi Compatibility"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define standards-oriented bidirectional browser automation and Selenium compatibility."
---

# WebDriver BiDi Compatibility

## Position

WebDriver BiDi is a first-class standards-oriented protocol surface. It complements CDP, enables event streaming, and supports Selenium and Web Platform Test automation without binding product semantics to Chromium internals.

## Priority modules

- session creation/status/subscription/end;
- browsing context create/get tree/navigate/reload/close/activate;
- script evaluate/call function/realm lifecycle/message;
- network events and interception according to certified level;
- storage cookies;
- input actions;
- browser/client windows where applicable;
- log events;
- permissions/emulation extensions only when standardized or clearly namespaced.

## Resource mapping

BiDi session → canonical client connection/session scope. Browsing contexts map to canonical pages/frames. Realms map to V8/Chromium execution contexts. Shared IDs are stable only within the documented lifetime.

## Event subscriptions

Translate canonical events into subscribed BiDi events. Apply context filters, event ordering, resume limitations, and backpressure. Do not emit page-sensitive event payloads that policy suppresses.

## Errors

Map canonical errors to BiDi protocol errors and include safe namespaced extension data when needed. Unsupported modules are discoverable and fail explicitly.

## Extensions

Project-specific capabilities use a collision-resistant namespace and never alter standard command meaning. Candidate extensions include engine/fallback metadata, semantic tree/delta, deterministic workflow controls, and trace references.

## WPT integration

Use the BiDi endpoint to run applicable Web Platform Tests. Keep product-specific test harness adaptations isolated and upstream-compatible where practical.

## Certification

Publish supported modules, command/event coverage, known limitations, Selenium versions, native/hybrid behavior, and test evidence. “BiDi supported” without a matrix is not an acceptable claim.
