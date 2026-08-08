---
title: "Chromium Compatibility Engine"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define Chromium worker pooling, isolation, protocol adaptation, resource policy, and fallback observability."
---

# Chromium Compatibility Engine

## Role

Chromium is a first-class compatibility engine, not a hidden emergency hack. It handles visual and unsupported browser behavior while preserving Project Machina session, policy, command, error, telemetry, and workflow contracts.

## Worker model

- Prewarmed browser processes grouped by version, region, isolation tier, proxy class, and feature profile.
- Browser contexts used only where tenant/isolation policy allows.
- Dedicated process or hardened container/microVM for stronger tiers.
- Maximum contexts, sessions, lifetime, memory, and crash thresholds.
- Graceful drain before version rollout or worker recycle.

## Adapter

The Chromium adapter translates canonical commands to supported CDP/automation operations and translates events/errors back to canonical forms. It must not expose internal CDP quirks as product semantics unless the public CDP endpoint requires them.

## Direct-routing cases

- screenshot, PDF, pixel/visual comparison;
- full CSS/layout/paint requirement;
- canvas, WebGL/WebGPU;
- audio/video/WebRTC;
- extension-dependent flow;
- compatibility mode requiring an unimplemented native API;
- site/domain policy configured for Chromium.

## Migration support

Use `STATE_BRIDGE.md`. Chromium workers accept a transfer bundle, apply configuration/state, navigate/replay, and verify the checkpoint before becoming active.

## Versioning

Pin Chromium builds. Roll out through canary, compatibility suite, corpus, and rollback gates. Store engine build/version in every trace and command outcome. Protocol client compatibility is tested against the pinned version, not assumed from “Chromium.”

## Resource controls

Apply process/container CPU, memory, PIDs, disk, network, download, and time limits. Disable unnecessary features where consistent with requested fidelity. A compatibility request does not bypass egress, secrets, approval, or tenant policy.

## Crash behavior

Classify browser process, renderer, GPU, network service, and protocol disconnection failures. Preserve redacted diagnostics, close affected contexts, and retry only when the command/idempotency policy permits. Repeated domain/build crashes trip a circuit breaker.

## Observability

Report:

- direct versus migrated launch;
- reason/capability ID;
- startup and migration latency;
- process/context resource usage;
- crashes and retries;
- version;
- state method and omissions;
- task verification result.
