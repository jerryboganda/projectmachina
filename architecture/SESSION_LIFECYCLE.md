---
title: "Session and Navigation Lifecycle"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define deterministic session, context, page, navigation, command, and close state transitions."
---

# Session and Navigation Lifecycle

## Session states

```text
REQUESTED -> QUEUED -> STARTING -> READY
READY <-> MIGRATING
READY -> CLOSING -> CLOSED
Any nonterminal -> FAILED
Any nonterminal -> EXPIRED
```

Transitions are persisted by the control plane and emitted as ordered events. Repeated close/cancel requests are idempotent.

## Navigation states

```text
CREATED
 -> REQUEST_STARTED
 -> RESPONSE_HEADERS
 -> PARSING
 -> DOM_INTERACTIVE
 -> SCRIPTS_RUNNING
 -> TASK_READY (zero or more named predicates)
 -> LOAD_COMPLETE
 -> QUIESCENT (optional)
 -> SUPERSEDED | FAILED | CANCELLED | CLOSED
```

`TASK_READY` is predicate-based and may occur before or after `LOAD_COMPLETE`. Commands specify their required state rather than relying on a global wait.

## Navigation identity

Each top-level and frame navigation has a unique navigation ID. Same-document history changes are separate lifecycle events but preserve document identity when standards semantics require it.

## Wait conditions

Supported conceptual waits:

- commit/response start;
- DOM interactive;
- load complete;
- network idle with explicit window and ignored resource classes;
- selector/role state;
- JavaScript predicate;
- semantic revision stable for a bounded interval;
- workflow checkpoint;
- custom conjunction/disjunction.

All waits require a deadline and return the observed condition and revisions.

## Command lifecycle

```text
ACCEPTED -> AUTHORIZED -> DISPATCHED -> RUNNING
 -> SUCCEEDED | FAILED | CANCELLED | DEADLINE_EXCEEDED
```

Retries create a new attempt under the same idempotency key when allowed. Events expose attempt number.

## Close semantics

Close stops accepting commands, cancels or drains according to policy, flushes bounded telemetry/artifacts, revokes temporary secrets, releases worker capacity, and persists final outcome. Hard kill is available after graceful deadline.

## Invariants

- No command runs before authorization and effective-policy resolution.
- A superseded navigation cannot complete a later wait accidentally.
- Events preserve per-session order even when transports reconnect.
- Deadlines use monotonic time internally.
- Worker disappearance leads to a classified terminal state, not an indefinitely queued session.
