---
title: "Unified Typed Command and Event Model"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define the canonical behavior contract shared by all engines, protocols, SDKs, workflows, and tests."
---

# Unified Typed Command and Event Model

## Purpose

The command model is the behavioral center of Project Machina. HTTP, gRPC, CDP, WebDriver BiDi, MCP, SDKs, workflows, native engine, and Chromium adapter all translate through it.

## Command envelope

```yaml
command_id: globally-unique
session_id: required
context_id: optional
page_id: optional
kind: navigation.goto.v1
payload: <typed by kind>
idempotency_key: optional/required by command class
deadline: RFC3339 or duration
expected_revision: optional
required_capabilities: []
verification:
  preconditions: []
  postconditions: []
metadata:
  correlation_id: ...
  causation_id: ...
  client: ...
```

## Command classes

- Session: create, inspect, cancel, close.
- Context/page/frame: create, list, close, switch.
- Navigation: goto, reload, history, wait.
- DOM/query: evaluate, query, snapshot, semantic tree/delta.
- Interaction: click, fill, press, select, check, upload.
- Network: intercept, continue, fulfill, fail, cookies.
- Storage: get/set/clear/export/import.
- Artifacts: screenshot/PDF through capable engine, trace, bundle.
- Workflow: record, compile, run, pause, approve.
- Diagnostics: capabilities, metrics snapshot, console/errors.

## Outcome envelope

```yaml
command_id: ...
attempt: 1
status: succeeded | failed | cancelled | deadline_exceeded
result: <typed by command>
error: <canonical error or null>
execution:
  requested_engine_policy: prefer-native
  engine: native
  engine_version: ...
  capability_snapshot: ...
  fallback:
    used: false
    reason: null
    migration_id: null
  revisions:
    document_before: 183
    document_after: 185
    semantic_before: 88
    semantic_after: 90
  verification:
    preconditions: passed
    postconditions: passed
  timings: {}
  resources: {}
trace_ref: optional
```

## Events

Every event has event ID, session sequence, type/version, session/context/page/navigation/command IDs as applicable, timestamp, causation/correlation, engine, payload, and classification. Clients resume streams with last acknowledged sequence.

Event families:

- session/worker lifecycle;
- navigation/document lifecycle;
- request/response/network;
- console/script/error;
- DOM/semantic revisions;
- interaction/action verification;
- capability/fallback/migration;
- workflow/approval;
- quota/resource/health.

## Idempotency classes

| Class | Behavior |
| --- | --- |
| Read-only | safe transport retry |
| Create with key | same key returns same resource/outcome |
| Set-to-value | retry allowed when version/precondition holds |
| Navigation | retry only under declared policy and no unsafe side effect |
| Side-effecting interaction | no automatic retry without workflow idempotency checkpoint |

## Capability declaration

Every command kind declares required and optional capabilities, eligible engines, approximation policy, and whether migration is allowed before/during execution. Capability IDs are stable names such as `dom.query.v1`, `visual.screenshot.v1`, or `network.intercept.v1`.

## Versioning

Additive fields are tolerated according to schema rules. Breaking semantic changes create a new command/event version. Adapters publish supported versions and do not reinterpret an old command silently.

## Testing

Each command requires:

- schema/serialization round trip;
- engine contract tests;
- canonical error tests;
- cancellation/deadline test;
- adapter translation tests;
- capability/fallback behavior;
- security/policy test where relevant.
