---
title: "Native Engine Architecture"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define the machine-oriented browser core, its internal modules, constraints, and phased capability strategy."
---

# Native Engine Architecture

## Purpose

The native engine implements the browser behavior required by machine workloads while intentionally omitting a full graphical rendering stack. It prioritizes standards-oriented DOM/JavaScript/network semantics, semantic interaction, deterministic extraction, startup speed, and session density.

## Composition

```text
NativeEngine
├── Session/Context/Page model
├── Navigation controller
├── Network loader and policy client
├── Streaming HTML tokenizer/tree builder
├── Compact DOM and mutation system
├── V8 runtime and Web IDL-style bindings
├── Event loop, tasks, microtasks, timers
├── Events, focus, forms, input, history
├── Fetch/XHR, cookies, storage, WebSocket/workers
├── Selector/XPath engine
├── Semantic visibility/interactability kernel
├── Extraction and schema engine
├── Capability instrumentation
└── Telemetry, budgets, cancellation, crash diagnostics
```

## Memory model

- Arena/region allocation for document-lifetime nodes and strings where safe.
- Generational node handles rather than exposed raw pointers.
- Interned element/attribute/namespace names.
- Small-vector storage for common children/attributes/listeners.
- Lazy JavaScript wrappers and derived semantic data.
- Bulk reset on document destruction.
- Explicit accounting for DOM, V8 heap, network buffers, storage, events, and trace data.
- Hard/soft budget thresholds with typed termination.

## Execution model

A worker owns one or more sessions according to isolation tier. Each session has deterministic command serialization rules, while page network and task queues use bounded asynchronous concurrency. V8 isolates/contexts and native objects have explicit thread affinity.

## Standards strategy

- Use Web Platform Tests by prioritized subsystem.
- Implement normative semantics needed by target workloads rather than browser-brand quirks by default.
- Track every API in the capability registry.
- Surface unsupported behavior immediately and structurally.
- Differential-test observable outcomes against Chromium.

## Omitted native rendering

The engine may compute a semantic subset of CSS and geometry needed for visibility, focus, hit testing, and interaction. It does not claim pixel equivalence. Screenshot, PDF, full layout, paint, fonts, canvas, GPU, and media route to Chromium.

## Capability phases

### Foundation

Navigation, HTML, DOM, V8, events, timers, fetch/XHR, cookies, web storage, selectors, extraction.

### Automation

Forms, keyboard/mouse, focus, history, Shadow DOM, custom elements, frames, dialogs, downloads, interception.

### Rich execution

Workers, WebSockets, service workers, IndexedDB, broader Web APIs based on target corpus.

## Robustness rules

- All page inputs are untrusted.
- Parser and bindings are fuzz targets.
- Unsafe code and FFI require documented invariants.
- No unbounded recursion on adversarial DOM/HTML.
- Script termination and cancellation are supported.
- A page failure is contained to its configured isolation boundary.
