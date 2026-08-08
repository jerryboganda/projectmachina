---
title: "V8 Integration and FFI Contract"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define ownership, snapshots, execution, termination, bindings, and security for the narrow V8 boundary."
---

# V8 Integration and FFI Contract

## Decision

Use V8 for JavaScript and WebAssembly. Keep V8-specific C++ code in `cpp/v8-bridge`; expose a small C-compatible ABI to a safe Rust facade in `crates/runtime-v8`.

## Boundary goals

- Hide C++ templates and V8 handles from the Rust workspace.
- Encode ownership and thread affinity in opaque handles.
- Convert exceptions and termination to typed results.
- Avoid unwinding exceptions across the C ABI.
- Make isolate/context lifecycle explicit.
- Support startup snapshots and shared read-only data.

## Conceptual handles

- `RuntimePlatformHandle`
- `IsolateHandle`
- `ContextHandle`
- `ScriptHandle` or cached-data handle
- `ValueHandle` valid only within declared scope
- `SnapshotHandle`

Each opaque handle has create/use/destroy functions and an owning thread/isolate identity. Stale or cross-isolate use fails deterministically in checked builds.

## Lifecycle

1. Initialize process-level V8 platform once.
2. Load verified custom startup snapshot.
3. Create isolate with allocator, heap constraints, callbacks, and telemetry.
4. Create context with browser globals/bindings.
5. Execute tasks under deadline/cancellation interrupt support.
6. Drain microtasks according to event-loop policy.
7. Dispose context, release native wrappers, then dispose isolate.
8. Shut down process platform only after all isolates are gone.

## Startup snapshot

Include stable, version-matched initialization:

- browser global scaffolding;
- DOM binding templates;
- fetch/event/workflow primitives;
- common serializers and extraction helpers;
- immutable built-ins safe to share.

Snapshot generation is deterministic and tied to exact V8/build flags. CI verifies that a runtime cannot load a mismatched snapshot.

## Native object bindings

- JavaScript wrappers reference generational native handles, not raw pointers.
- Wrapper finalization releases references through a safe queue on the owning runtime.
- Native node destruction invalidates handles; JavaScript access receives a detached/stale-object error according to API semantics.
- Security-sensitive properties and cross-origin access pass through centralized checks.

## Deadlines and termination

Command deadline or session cancellation requests V8 termination through its supported interrupt/termination mechanism. The event loop classifies terminated execution, cleans up task state, and determines whether the context remains reusable. Infinite microtask chains and promise storms are budgeted.

## Error translation

Capture exception type, message, sanitized stack, source location, promise rejection state, and cause where available. Page exceptions are events unless the command contract makes them fatal. Never return C++ exception text as the only error code.

## Security and testing

- Build sanitizer variants for bridge testing.
- Fuzz argument conversion, wrapper lifetime, exceptions, snapshots, and teardown.
- Run V8 security updates through expedited dependency policy.
- Audit every new C++ ABI function for ownership, thread, exception, and buffer semantics.
