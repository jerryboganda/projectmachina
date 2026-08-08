---
title: "Data Flow and Classification"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Map browser, tenant, secret, telemetry, and artifact data through the platform."
---

# Data Flow and Classification

## Data classes

| Class | Examples | Default handling |
| --- | --- | --- |
| Public | published docs, capability matrix | ordinary integrity/access controls |
| Internal | build metadata, fleet health | authenticated staff/system access |
| Tenant confidential | URLs, DOM-derived output, cookies, workflow inputs | tenant isolation, encryption, short retention defaults |
| Secret | passwords, tokens, proxy credentials, vault values | opaque references, moment-of-use resolution, never ordinary logs |
| Security sensitive | sandbox policy, audit details, crash internals | restricted access and retention |
| Regulated/unknown page data | user-provided site content | minimize, classify conservatively, tenant policy |

## Session data path

```text
Client request
 -> API authentication/authorization
 -> policy resolution
 -> scheduler assignment
 -> worker configuration
 -> outbound network through policy/proxy
 -> hostile page responses
 -> parser/V8/Chromium execution
 -> command outcome / semantic output
 -> redaction and response
 -> metering + typed telemetry
 -> optional classified artifact
```

## Secret path

```text
Workflow contains secret reference
 -> policy authorizes reference for project/action
 -> worker requests scoped value over authenticated channel
 -> value held in protected memory for bounded duration
 -> value entered into page
 -> logs/recording retain reference and redaction marker only
 -> memory cleared/released according to runtime limits
```

## State migration path

- Export only allowlisted transferable fields.
- Encrypt transport and bind bundle to tenant, session, destination, expiry, and nonce.
- Apply origin and policy constraints.
- Verify destination state using non-secret checks.
- Destroy temporary bundle after successful import or expiry.
- Record method: direct transfer, navigation replay, action replay, or partial transfer.

## Telemetry minimization

Default metrics use dimensions such as engine, capability ID, error code, timing bucket, resource bucket, and hashed/normalized domain category. Raw URL, DOM, request body, headers, cookies, form values, console payloads, and screenshot content require explicit capture policy and retention.

## Artifact access

Every artifact has tenant/project ownership, purpose, classification, creation/expiry, checksum, encryption key reference, and audit trail. Signed download URLs are short lived and scope checked.
