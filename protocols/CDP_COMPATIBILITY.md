---
title: "Chrome DevTools Protocol Compatibility"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define a pinned, evidence-backed CDP subset for Playwright, Puppeteer, and direct clients."
---

# Chrome DevTools Protocol Compatibility

## Position

CDP is an adapter for ecosystem compatibility, not the native internal API. The upstream tip-of-tree protocol changes frequently; Project Machina pins schema revisions and certifies explicit client/runtime combinations.

## Endpoint

Expose a WebSocket endpoint that supports target/session discovery and attachment according to the certified subset. Authentication and tenant scoping occur before protocol upgrade.

## Priority domains

### P0

- `Browser` minimal version/capability operations.
- `Target` creation/attachment/close for contexts/pages.
- `Page` navigation/lifecycle/frame tree/evaluate-related integration.
- `Runtime` evaluate/call/exception/console object subset.
- `DOM` query/describe/attributes subset.
- `Network` events, cookies, interception subset.
- `Input` mouse/keyboard/touch subset.
- `Emulation` viewport/locale/timezone/user agent subset.
- `Log`/console events.

### P1

Downloads, permissions, tracing, performance, storage, workers, dialogs, more complete interception, device/mobile emulation, and capabilities required by certified Playwright/Puppeteer releases.

### Chromium-only

Screenshot/PDF and visual/rendering-specific operations may execute only on Chromium but remain available through the endpoint when policy allows.

## Unsupported behavior

For a known unsupported command:

- return a protocol error with Project Machina canonical code in structured data when possible;
- never return `{}` success unless empty success is the documented semantics;
- allow automatic migration only if the command/session policy permits and the protocol interaction remains coherent;
- record compatibility telemetry.

## Client certification

For each supported Playwright and Puppeteer version, run a matrix covering launch/connect, contexts, pages, frames, navigation, evaluation, selectors/actions, cookies/storage, interception, dialogs, downloads, parallelism, and close/error behavior. Publish pass/limited/Chromium-only/unsupported.

## Session mapping

CDP target/session IDs map to canonical session/context/page/frame resources. Detach and reconnect behavior is explicit. A migration preserves the external logical target where feasible and emits relevant lifecycle changes; otherwise it returns a classified disconnection rather than hiding replacement.

## Schema generation

Generate protocol types/dispatch tables from the pinned schema, then layer hand-written semantic adapters. Preserve unknown events/fields according to compatibility policy; do not accept arbitrary unvalidated method names.
