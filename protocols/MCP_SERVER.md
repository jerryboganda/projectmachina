---
title: "Model Context Protocol Server"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define secure, typed, low-token browser tools, resources, prompts, and event behavior for AI agents."
---

# Model Context Protocol Server

## Objective

Expose Project Machina as an efficient agent tool surface. Prefer semantically compact native commands over raw CDP/DOM dumps, while retaining explicit engine, verification, and fallback information.

## Tool groups

### Session

- `browser_session_create`
- `browser_session_status`
- `browser_session_close`

### Navigation and wait

- `browser_goto`
- `browser_wait`
- `browser_back` / `browser_forward` / `browser_reload`

### Observe

- `browser_semantic_snapshot`
- `browser_semantic_delta`
- `browser_extract`
- `browser_console_errors`
- `browser_capabilities`

### Act

- `browser_click`
- `browser_fill`
- `browser_press`
- `browser_select`
- `browser_check`
- `browser_upload` with policy/approval

### Workflow and diagnostics

- `workflow_record_start/stop`
- `workflow_compile/run/status`
- `approval_respond`
- `trace_summary`
- `reproduction_bundle_create`

## Tool design

- Small typed inputs with stable IDs and semantic locators.
- Bounded outputs with pagination/delta and an explicit truncation marker.
- No secret values returned.
- Every action returns pre/postcondition, revisions, navigation/fallback, and safe next-observation hint.
- Long operations return progress/resource identifiers rather than blocking indefinitely.

## Resources

Read-only resources may expose capability matrix, session summary, workflow definition/version, trace summary, and approved artifact metadata. Access is tenant/policy scoped.

## Prompts

Optional prompt templates help an agent discover then compile a workflow, diagnose a failed action, or select a fidelity profile. Prompts do not replace enforcement.

## Security

- Authenticate and scope MCP connections.
- Require policy evaluation for every tool call.
- Apply approval gates to configured high-impact actions.
- Bound sessions, tokens/output, page text, artifacts, and network access.
- Treat model-provided locator/text/URL as untrusted input.
- Defend against page prompt injection by separating page observations from system policy and never allowing page content to alter tool permissions.

## Token efficiency

Default observations return interactive semantic deltas and concise errors, not full DOM/HTML. Clients request larger snapshots explicitly. Repeated workflow execution returns step summaries unless diagnostics are enabled.

## Protocol version

Pin and test against the chosen stable MCP specification version. Keep transport/protocol adaptation separate from command behavior so future specification upgrades do not change browser semantics.
