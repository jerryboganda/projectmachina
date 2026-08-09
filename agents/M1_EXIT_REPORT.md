# M1 Compatibility-First Exit Report

## Status

`BLOCKED_EXTERNAL_RUNTIME_AND_LISTENER_INTEGRATION`

M1 source contracts and deterministic compatibility evidence are implemented.
M1 is **not** declared exited because the owner-selected Docker waiver still
prevents a real Chromium process/container, HTTP listener, gRPC listener, and
end-to-end SDK journey against those listeners from being certified.

## Passing injected command-core smoke

Command:

```text
node --test scripts/test/m1-compatibility-smoke.test.mjs
```

The smoke harness passed the same canonical command shapes with four labeled
surface modes:

- HTTP contract label
- gRPC contract label
- TypeScript SDK contract label
- Python SDK contract label

These are **not live HTTP/gRPC/SDK clients**. They intentionally use one
injected command core so the canonical shape, routing metadata, event sequence,
and error/restart behavior can be checked without faking unavailable listeners
or a browser process. The loopback fixture form endpoint is separately sanity
checked once. The per-label journey covers session creation, fixture
navigation, semantic extraction, ordered event reconnect, and idempotent close.

The same run also passed:

- explicit `COMMAND_CANCELLED`;
- explicit `UNSUPPORTED_CAPABILITY`;
- explicit `WORKER_LOST`;
- injected worker restart;
- control-plane snapshot/restart reconciliation;
- one trace reference per injected session with request and verified
  worker-outcome events.

## Runtime limitation

The smoke intentionally does not fake a successful Chromium process. The
adapter reports runtime/capability state explicitly, and the real process,
listener, client/server, and crash evidence remains blocked by `BLK-003` and
the owner Docker waiver in `agents/WAIVERS.md`.

## Native-ready interfaces

The command bus, capability registry/router, ordered event broker, trace
context, artifact access contract, and SDK transports are transport-neutral.
The native engine can be attached through the same `EngineAdapter` and
capability snapshot interfaces without changing public command semantics.
This report does not claim that those separate modules have been composed into
live listener processes.

## Heavy validation

Real Chromium/browser execution, listener-level HTTP/gRPC interoperability,
multi-process client disconnect, worker crash recovery, broad protocol
conformance, load, soak, chaos, WPT, and certification suites remain owned by
the runtime-enabled M1/M4/M8/M9 gates. No production or container-readiness
claim follows from this report.
