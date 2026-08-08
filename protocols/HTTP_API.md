---
title: "HTTP and JSON API Contract"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define the public resource-oriented API, authentication, idempotency, errors, pagination, streaming, and compatibility."
---

# HTTP and JSON API Contract

## Style

Use versioned HTTPS JSON endpoints for control and command operations. Generate OpenAPI from or alongside the canonical schemas and validate generated SDKs against contract tests.

## Base resources

```text
/v1/organizations
/v1/projects
/v1/projects/{project_id}/sessions
/v1/sessions/{session_id}
/v1/sessions/{session_id}/commands
/v1/sessions/{session_id}/events
/v1/sessions/{session_id}/artifacts
/v1/projects/{project_id}/workflows
/v1/workflows/{workflow_id}/versions
/v1/workflow-runs
/v1/approvals
/v1/capabilities
/v1/usage
```

## Authentication and authorization

Support project API credentials and user/session authentication appropriate to console use. The exact method is deployment-specific, but every request resolves principal, organization, project, roles/scopes, and policy before resource access. Do not encode authorization solely in opaque client-side routing.

## Session creation example

```http
POST /v1/projects/prj_123/sessions
Idempotency-Key: 8b6...
Content-Type: application/json

{
  "engine_policy": "prefer-native",
  "fidelity_profile": "agent",
  "isolation_tier": "dedicated-process",
  "deadline_seconds": 300,
  "network_policy_id": "np_123",
  "proxy_secret_ref": null,
  "locale": "en-US",
  "timezone": "UTC"
}
```

Returns `201` when ready synchronously or `202` with lifecycle URL when queued/starting. Idempotency returns the original resource for the same normalized request and key.

## Commands

`POST /v1/sessions/{id}/commands` accepts the canonical command envelope or a simplified endpoint-specific representation. Long commands return an operation resource or stream events. Clients may cancel through an idempotent cancel endpoint.

## Event streaming

Provide Server-Sent Events for broad HTTP compatibility and WebSocket only where bidirectional streaming adds value. Event sequence supports resume through `Last-Event-ID` or explicit cursor. gRPC remains preferred for high-volume typed streams.

## Pagination

Cursor-based pagination with stable sort key. Cursors are opaque, scoped, expiring, and policy checked. Responses include `next_cursor`; absence means complete.

## Errors

Use canonical error envelope and suitable status:

- 400 validation;
- 401 unauthenticated;
- 403 policy/authorization;
- 404 scoped resource absence;
- 409 state/version/idempotency conflict;
- 422 semantically invalid command;
- 429 quota/rate;
- 5xx infrastructure/transient.

HTTP status alone is not the application error contract.

## Concurrency control

Mutable durable resources expose version/ETag. Update accepts `If-Match` or expected version. Session commands use sequence/revision preconditions where applicable.

## Security

Apply size limits, content types, schema validation, request deadlines, CSRF protection for browser-authenticated mutations, CORS policy, secure headers, audit, and no secret echo. Signed artifact downloads are short lived and authorization checked at issuance.
