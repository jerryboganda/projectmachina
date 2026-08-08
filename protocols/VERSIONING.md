---
title: "API, Protocol, Schema, and Capability Versioning"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define compatibility guarantees, deprecation, migrations, and pinned ecosystem versions."
---

# Versioning

## Version dimensions

- Product/server release.
- Native engine build and capability snapshot.
- Chromium build.
- Command/event schema version.
- HTTP API major version.
- gRPC protobuf revision.
- CDP schema revision and certified client versions.
- WebDriver BiDi specification/module support.
- MCP specification/server version.
- SDK package version.
- Workflow DSL/schema version.
- State-transfer bundle version.

## Rules

- Stable public HTTP breaking changes require a new major path/version or documented migration.
- Protobuf fields are added compatibly; removed numbers/names are reserved.
- Breaking command semantics create a new command version.
- Capability status is tied to engine build/config and may progress or regress only with evidence and release notes.
- Workflow and state-transfer formats include explicit version and migration code.
- CDP is pinned; do not claim generic tip-of-tree compatibility.
- BiDi/MCP support names the specification revision used.

## Deprecation

A deprecation notice includes replacement, first-deprecated version/date, minimum support window, telemetry, migration guide, and removal release. Security emergencies may shorten the window with owner approval and communication.

## Compatibility testing

Run previous supported SDK against new server, new SDK against previous supported server where promised, stored workflow versions against new runtime, state bundle migrations, and certified ecosystem client matrix.

## Release metadata

Every response/trace exposes sufficient server/engine/capability version metadata for reproduction without revealing infrastructure-sensitive details.
