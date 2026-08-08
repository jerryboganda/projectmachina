---
title: "Observability and Reproduction Architecture"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define metrics, logs, traces, profiles, audit, redaction, and failure reproduction across engines."
---

# Observability and Reproduction Architecture

## Signals

### Metrics

Low-cardinality counters, histograms, and gauges for sessions, commands, engine choice, capability misses, fallback, latency, CPU, memory, requests/bytes, queueing, crashes, errors, retries, workflows, approvals, and fleet health.

### Traces

Distributed traces connect API request, session scheduling, command dispatch, navigation, network, V8/DOM work, state migration, Chromium operations, workflow steps, and artifact generation.

### Logs

Structured events with timestamp, severity, component, build, tenant/project/session pseudonymous IDs, correlation/causation, code, and redaction classification. No arbitrary page payloads by default.

### Profiles

Opt-in or sampled CPU/heap/allocation profiles, bounded and classified. Production profiling requires policy and access control.

### Audit

Immutable or tamper-evident records for authentication, authorization changes, secrets, approvals, privileged operations, artifact access, policy changes, releases, and emergency actions.

## Correlation model

Use stable IDs:

- request ID;
- session/context/page/navigation ID;
- command ID and attempt;
- trace/span ID;
- workflow/run/step ID;
- migration ID;
- worker/build ID;
- artifact ID.

## Reproduction bundle

A bundle may include:

- product/engine/protocol/client versions;
- effective policy and fidelity profile with secrets removed;
- command/event timeline;
- network metadata and optional approved bodies;
- console/page errors;
- semantic/DOM revisions or approved snapshots;
- action history and workflow checkpoint;
- capability/fallback decisions;
- resource metrics;
- crash/sanitizer data;
- exact replay/test command;
- checksums and classification.

## Redaction

Central redaction runs before storage/export and understands headers, cookies, query fields, form values, secret references, tokens, URLs, DOM text policies, console payloads, and stack data. Test redaction with seeded canary secrets.

## Sampling

- Metrics: unsampled aggregates.
- Errors/crashes/migrations: high or full sampling subject to privacy.
- Successful traces: adaptive sample by workload/tenant policy.
- Page content: off by default; explicit diagnostic capture with short retention.

## SLO usage

User-facing SLOs derive from server-side verified outcomes. Dashboard metrics cannot count a command as successful before postcondition verification.
