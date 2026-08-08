---
title: "gRPC and Event Streaming Contract"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define efficient typed command, lifecycle, and event streams for high-throughput clients and internal services."
---

# gRPC and Event Streaming Contract

## Goals

- Strongly typed high-throughput command/event interface.
- Bidirectional stream for sessions that need low-latency action and events.
- Stable schema evolution and generated clients.
- Deadlines, cancellation, backpressure, resume, and ordered session events.

## Services

```proto
service SessionService {
  rpc CreateSession(CreateSessionRequest) returns (Session);
  rpc GetSession(GetSessionRequest) returns (Session);
  rpc CancelSession(CancelSessionRequest) returns (Session);
  rpc CloseSession(CloseSessionRequest) returns (Session);
  rpc Execute(CommandRequest) returns (CommandOutcome);
  rpc Connect(stream ClientFrame) returns (stream ServerFrame);
}

service WorkflowService { ... }
service CapabilityService { ... }
service ArtifactService { ... }
```

## Bidirectional frames

Client frames include command, cancel, acknowledge sequence, heartbeat, and flow-control hints. Server frames include outcome, event, warning, server heartbeat, and stream-control messages.

## Ordering

Commands may be pipelined only when their command kind declares safe concurrency. Outcomes correlate by command ID. Session events carry a monotonically increasing sequence. Transport order is not a substitute for semantic order across reconnects.

## Backpressure

- Bound unacknowledged events and bytes.
- Pause optional high-volume event categories before dropping critical lifecycle/error events.
- Send `RESYNC_REQUIRED` if a cursor falls outside retention.
- Client SDKs expose bounded async iterators/channels.

## Deadlines and cancellation

Use gRPC deadlines translated into canonical command deadlines. Stream cancellation cancels subscriptions, not automatically the browser session unless requested. Command cancellation has a dedicated frame/ID.

## Schema evolution

- Reserve removed field numbers and enum values.
- Add fields compatibly; do not change meaning.
- Version commands/events where semantics break.
- Run backward/forward descriptor compatibility checks in fast gates.
- Keep protobuf definitions canonical and generate gateway/OpenAPI types where appropriate.

## Internal events

Control-plane event bus uses a related but not necessarily public protobuf envelope. Public events are sanitized projections. Both include event ID, aggregate sequence, schema version, causation/correlation, tenant scope, and classification.
