---
title: "Component Boundaries and Dependency Rules"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Prevent architectural erosion and duplicate browser semantics across agents and protocols."
---

# Component Boundaries and Dependency Rules

## Layer model

| Layer | Owns | Must not own |
| --- | --- | --- |
| Foundation | IDs, time, cancellation, bounded queues, serialization helpers | Browser semantics |
| Command contracts | Typed commands/events/errors/capability metadata | Engine implementation or transport |
| Browser model | DOM, navigation, events, storage, network semantics | Public transport/auth |
| Engine composition | Native and Chromium execution adapters | Tenant/business policy storage |
| Routing/session | Policy, engine choice, migration coordination | Duplicate page semantics |
| Protocol adapters | Wire schemas and translation | Independent behavior or silent approximation |
| Control plane | Organizations, projects, policies, workflows, durable lifecycle | Execute hostile page code |
| Frontend/SDK | User experience and ergonomic clients | Authorization decisions or duplicated contracts |
| Operations | Deployment, telemetry, fleet control | Product-level behavior hidden from APIs |

## Boundary contracts

### Command model

The only supported route from public adapter to browser behavior. It defines command IDs, inputs, outputs, events, deadlines, cancellation, errors, capability requirements, verification metadata, and idempotency class.

### Engine interface

Both native and Chromium engines implement an internal trait/facade such as:

```rust
trait EngineSession {
    async fn execute(&self, command: Command, ctx: CommandContext)
        -> Result<CommandOutcome, CommandError>;
    fn capabilities(&self) -> CapabilitySnapshot;
    async fn export_state(&self, request: StateExportRequest)
        -> Result<TransferBundle, StateError>;
    async fn close(&self, reason: CloseReason) -> Result<(), CloseError>;
}
```

Exact code may differ, but no protocol-specific type crosses this boundary.

### State bridge

Consumes versioned transferable state and verified action history; it never reads arbitrary private engine memory.

### Telemetry

Components emit typed, redaction-aware events. Logging macros or SDKs must apply context and classification centrally.

## Forbidden dependencies

- Native engine importing CDP/BiDi/MCP types.
- Protocol adapter bypassing authorization or calling V8/DOM directly.
- Frontend depending on database schema.
- Worker writing control-plane database records directly except through a scoped service contract.
- Capability router inferring support from error strings.
- Tests modifying production semantics solely to make comparison pass.
- Agent workflow runtime executing privileged actions without policy evaluation.

## Boundary-change rule

A change to a public or cross-component boundary requires:

1. ADR or documented compatible extension.
2. Contract/schema update.
3. Generated clients/bindings update.
4. Compatibility and migration note.
5. Contract test.
6. Capability/version update.
7. Independent architecture/protocol review.
