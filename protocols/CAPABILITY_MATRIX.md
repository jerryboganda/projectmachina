---
title: "Capability and Compatibility Matrix"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define the source-of-truth format for native, hybrid, Chromium, protocol, client, and evidence status."
---

# Capability and Compatibility Matrix

## Status values

- `native`: implemented and certified in native engine.
- `native-limited`: implemented with documented constraints.
- `hybrid`: may migrate or route based on runtime need.
- `chromium`: available only through compatibility engine.
- `experimental`: opt-in and not certified.
- `unsupported`: explicit error.
- `disabled-by-policy`: engine could perform it, current policy cannot.

## Initial planning matrix

| Capability family | Beta target | GA target | Evidence |
| --- | --- | --- | --- |
| HTTP navigation/redirects | native | native | WPT + corpus |
| HTML/DOM/selectors | native | native | WPT + differential |
| JavaScript/WebAssembly | native | native | engine/compat suite |
| Fetch/XHR/cookies/web storage | native | native | WPT + differential |
| Forms/focus/keyboard/mouse | native-limited | native | WPT + task corpus |
| Shadow DOM/custom elements | native-limited | native | WPT |
| Frames/history | native-limited | native | WPT + clients |
| Workers/WebSockets | hybrid | native-limited/native | WPT + corpus |
| Service worker/IndexedDB | hybrid | native-limited | WPT + corpus |
| Network interception | hybrid | native-limited | client conformance |
| Downloads/uploads/dialogs | hybrid | hybrid/native-limited | client conformance |
| Semantic tree/delta/extraction | native | native | product tests |
| Deterministic workflows | native | native | workflow suite |
| Screenshot/PDF/visual | Chromium | Chromium | Chromium conformance |
| Canvas/GPU/media/WebRTC | Chromium | Chromium | Chromium conformance |

## Required evidence fields

```yaml
capability_id: dom.query.v1
status: native
engine_build: ...
protocol_surfaces: [http, grpc, cdp, bidi, mcp]
client_versions: []
limitations: []
tests:
  - suite: wpt
    revision: ...
    result_artifact: ...
last_verified: ...
owner: ...
```

## Publication rule

Generate public documentation from the registry used by routing and tests. No manually maintained marketing matrix may contradict runtime capability responses.
