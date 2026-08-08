---
title: "Protocol and Client Conformance Testing"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Certify HTTP, gRPC, CDP, BiDi, MCP, SDKs, and ecosystem clients against explicit versions and limitations."
---

# Protocol and Client Conformance Testing

## Test layers

- Schema/descriptor compatibility.
- Serialization and canonical translation.
- Protocol state machine and lifecycle.
- Engine-independent contract fixtures.
- Native, Chromium, and migration behavior.
- Ecosystem client end-to-end suites.
- Error, cancellation, reconnect, backpressure, and unsupported behavior.

## Matrices

### HTTP/gRPC

Current and supported previous API/SDK versions; idempotency, pagination, streaming, deadlines, auth, errors, and event resume.

### CDP

Pinned protocol revision with selected Playwright and Puppeteer releases. Record domain/method/event coverage and native/Chromium behavior.

### WebDriver BiDi

Named specification revision/modules with Selenium and direct client versions.

### MCP

Named stable specification revision, supported transports, tool schemas, cancellation/progress, authorization, and bounded outputs.

### SDKs

Language/runtime versions, sync/async variants, examples, packaging/install, errors, retries, events, and resource cleanup.

## Negative tests

Malformed frames, unknown methods, invalid IDs, out-of-order commands, excessive payloads, unauthorized resources, stale cursors, duplicate idempotency, disconnect mid-command, slow readers, and unsupported capabilities.

## Certification artifact

For each tuple of server build, protocol revision, client version, engine policy, and isolation class, publish pass/limited/unsupported with linked test results and limitations. Do not infer certification from unit tests alone.
