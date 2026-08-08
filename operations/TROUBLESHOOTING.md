---
title: "Troubleshooting Guide"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Provide systematic diagnosis by symptom across clients, control plane, scheduler, native engine, Chromium, and data services."
---

# Troubleshooting Guide

## Diagnostic order

1. Capture correlation, session, command, build, engine policy, and region.
2. Confirm client request and canonical error code.
3. Inspect session lifecycle and scheduler/worker assignment.
4. Inspect engine/fallback/migration timeline.
5. Check resource budgets, network policy, and page/runtime errors.
6. Reproduce with approved bundle or deterministic fixture.
7. Compare current and previous known-good build/config.

## Symptom map

### `SESSION_NOT_READY` or long queue

Quota/capacity class, region/isolation mismatch, stale lease, autoscaler, worker image availability.

### Generic timeout observed by client

Verify server canonical error and deadline phase. Check navigation predicate, network, V8 task/microtask, command queue, event-stream delivery, and client-side timeout. Do not merely increase timeout.

### Native works but protocol client fails

Inspect adapter translation, target/context mapping, unsupported command, event subscription/order, schema/client version, and canonical direct command test.

### CLI/native differs from Chromium

Run minimized differential fixture; compare lifecycle, DOM/semantic revisions, cookies/storage, network and page errors. Classify standards or capability gap.

### Migration fails

Check policy eligibility, transferable categories, bundle version/integrity/expiry, destination availability, navigation/action replay, and checkpoint verification.

### Memory growth

Separate V8 heap, external/native DOM, network buffers, storage/cache, events/timers, traces/artifacts, Chromium processes, and allocator fragmentation. Reproduce under soak and inspect worker recycle.

### Svelte console stale/missing events

Check event sequence/cursor, authorization, reconnect/backpressure, generated client version, bounded buffer, server projection, and browser console/network. Session truth remains server-side.

## Reproduction discipline

Use exact version/config, minimal fixture, one failing command, expected/actual, logs/traces within policy, and repeat count. Add a regression test before closing product defect.
