---
title: "Language SDK Design"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define consistent, ergonomic, generated-plus-handwritten SDKs for TypeScript, Python, Go, Java, and Rust."
---

# Language SDK Design

## Supported SDKs

- TypeScript and Python: production beta.
- Go, Java, and Rust: GA target.

## Architecture

Each SDK has:

1. generated models and low-level client from canonical schemas;
2. handwritten ergonomic session/page/workflow API;
3. retry, deadlines, cancellation, pagination, event reconnect, and telemetry middleware;
4. examples and compatibility tests;
5. optional framework integrations.

Generated code is never edited manually.

## Object model

```text
MachinaClient
  -> ProjectClient
  -> Session
      -> Context
      -> Page
          -> Locator/SemanticLocator
          -> Events
  -> WorkflowClient
  -> CapabilityClient
```

## Async behavior

Use native language conventions:

- TypeScript promises/async iterables/AbortSignal.
- Python `asyncio`, async context managers/iterators; optional synchronous facade only if maintainable.
- Go contexts/channels or iterators with bounded buffering.
- Java futures/reactive stream or clear blocking/async variants.
- Rust async traits/streams with cancellation tokens.

## Errors

Expose typed canonical error classes/enums with code, retryability, safe details, correlation ID, engine/fallback metadata, and original transport status. Do not force users to parse strings.

## Retry

SDK defaults retry transient transport/capacity errors for safe commands. Side-effecting commands require idempotency or explicit user retry. Expose retry policy and attempt events.

## Resource management

Sessions and streams support `close`/context-manager patterns. SDKs send close when possible but server expiration remains authoritative. Avoid finalizer-only cleanup.

## Versioning

SDK major versions align with breaking public API changes, not necessarily server releases. Publish minimum/maximum tested server and protocol versions. Additive server fields must not break older clients.

## Examples

Every SDK ships:

- create session and extract;
- semantic interaction;
- explicit fallback policy;
- event/trace handling;
- workflow recording/replay;
- approval and secret-reference use;
- error and cancellation handling.
