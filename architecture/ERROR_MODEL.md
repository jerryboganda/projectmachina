---
title: "Canonical Error Model"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define stable typed failures, retryability, causality, and adapter translation."
---

# Canonical Error Model

## Goals

Errors are actionable, versioned, machine-readable, safe to expose, and consistent across HTTP, gRPC, CDP, BiDi, MCP, SDKs, native, and Chromium paths.

## Shape

```json
{
  "code": "ELEMENT_NOT_INTERACTABLE",
  "category": "interaction",
  "message": "The resolved element is not currently interactable.",
  "retryable": true,
  "retry_after_ms": null,
  "engine": "native",
  "capability": "interaction.click.v1",
  "command_id": "...",
  "correlation_id": "...",
  "details": {"state": ["hidden"]},
  "cause_code": null,
  "documentation_ref": "errors/ELEMENT_NOT_INTERACTABLE"
}
```

Messages are not stable API; codes and documented fields are.

## Categories and examples

### Request/auth/policy

`INVALID_ARGUMENT`, `UNAUTHENTICATED`, `PERMISSION_DENIED`, `POLICY_DENIED`, `QUOTA_EXCEEDED`, `RATE_LIMITED`, `REGION_UNAVAILABLE`.

### Lifecycle/capacity

`SESSION_NOT_READY`, `SESSION_CLOSED`, `SESSION_EXPIRED`, `CAPACITY_UNAVAILABLE`, `WORKER_LOST`, `COMMAND_CANCELLED`, `DEADLINE_EXCEEDED`.

### Capability/fallback

`UNSUPPORTED_CAPABILITY`, `CAPABILITY_DISABLED`, `RENDERER_REQUIRED`, `FALLBACK_PROHIBITED`, `MIGRATION_FAILED`, `STATE_TRANSFER_PARTIAL`, `APPROXIMATION_NOT_ALLOWED`.

### Navigation/network

`INVALID_URL`, `DNS_FAILED`, `TLS_FAILED`, `PROXY_AUTH_FAILED`, `NETWORK_POLICY_BLOCKED`, `REDIRECT_LIMIT`, `REQUEST_BUDGET_EXCEEDED`, `RESPONSE_TOO_LARGE`, `NAVIGATION_FAILED`.

### Script/DOM/interaction

`JAVASCRIPT_EXCEPTION`, `SCRIPT_TERMINATED`, `HEAP_LIMIT_EXCEEDED`, `SELECTOR_INVALID`, `ELEMENT_NOT_FOUND`, `ELEMENT_AMBIGUOUS`, `ELEMENT_DETACHED`, `ELEMENT_NOT_INTERACTABLE`, `ACTION_POSTCONDITION_FAILED`.

### Storage/workflow/artifact

`STORAGE_QUOTA`, `PROFILE_LOCKED`, `WORKFLOW_INVALID`, `WORKFLOW_DIVERGED`, `APPROVAL_REQUIRED`, `SECRET_UNAVAILABLE`, `ARTIFACT_EXPIRED`.

## Retry policy

Every code documents retryability and idempotency constraints. SDKs retry only transient transport/capacity failures for idempotent commands, with bounded exponential backoff and jitter. They never automatically retry side-effecting actions without an idempotency contract.

## Adapter mapping

Adapters map canonical errors to protocol-native forms while preserving canonical code in structured metadata where possible. A protocol limitation may reduce detail but cannot turn failure into success.

## Causality

Wrap lower-level causes with safe canonical context. Preserve internal diagnostic chain in restricted traces, not public messages. Redact URLs, headers, values, and stacks according to policy.
