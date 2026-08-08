---
title: "Capability and Fidelity Router"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Specify how the platform chooses native execution, Chromium, migration, or explicit rejection."
---

# Capability and Fidelity Router

## Inputs

- requested command and declared capability set;
- session fallback mode and fidelity profile;
- native/Chromium build capability snapshots;
- page/runtime observations;
- domain optimization history without sensitive content;
- isolation, region, proxy, and resource constraints;
- current state-transfer feasibility;
- cost/latency policy.

## Outcomes

| Outcome | Meaning |
| --- | --- |
| `native` | Execute entirely in native engine |
| `chromium-direct` | Full-browser need is known before start |
| `migrate` | Native attempt encountered an eligible capability miss |
| `reject` | Policy forbids fallback or no engine can satisfy requirement |
| `approximate` | Only when a documented approximation is requested and response is flagged |

## Fallback modes

- `native-only`: never launch Chromium; return typed unsupported/disabled error.
- `prefer-native`: start native when eligible and migrate when allowed.
- `prefer-compatible`: use Chromium when uncertainty exceeds configured threshold.
- `chromium-only`: bypass native path.

## Fidelity profiles

### `extract`

Block media/fonts, avoid image decode, limit frames, apply semantic CSS subset, favor task-ready waits, and cap requests/bytes aggressively.

### `agent`

Enable JavaScript, forms, focus, same-origin frames, semantic kernel, selected workers/WebSockets, and verified actions.

### `test`

Enable broader APIs, frame/worker behavior, interception, downloads, dialogs, contexts, emulation, and stricter compatibility.

### `visual`

Route directly to Chromium for screenshots, PDF, full layout, paint, canvas, GPU, or visual comparison.

### `custom`

Validated explicit resource/capability manifest.

## Decision record

Every decision emits:

```json
{
  "decision": "native|chromium-direct|migrate|reject|approximate",
  "reason_code": "CAPABILITY_ID_OR_POLICY_CODE",
  "router_version": "...",
  "capability_snapshot": "...",
  "confidence": "declared|deterministic|heuristic",
  "state_method": "none|transfer|replay|partial",
  "estimated_cost_class": "...",
  "actual_engine": "..."
}
```

## Runtime capability miss

Native code must surface a structured capability ID and context. Error strings are diagnostic only. The router evaluates migration policy; it never treats a generic timeout as automatic proof that Chromium is needed.

## Optimization loop

Aggregate fallback by capability ID, domain category, workload, and version. Prioritize native implementation where expected saved cost and user value justify complexity. Domain hints may alter prediction but not bypass security or misrepresent support.

## Safety rules

- No downgrade of isolation or egress policy during fallback.
- No migration if secret/state policy forbids export.
- No action replay beyond the last verified checkpoint without idempotency analysis.
- No approximation in certified compatibility mode unless explicitly allowed.
