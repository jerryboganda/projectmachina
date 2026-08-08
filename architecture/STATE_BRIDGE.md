---
title: "Native-to-Chromium State Bridge"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define safe transferable state, verified replay, migration checkpoints, and failure behavior."
---

# Native-to-Chromium State Bridge

## Goal

Continue a task in Chromium with the closest safe, observable, and policy-compliant state when native execution encounters an unsupported requirement.

## Transferable categories

| Category | Method | Constraints |
| --- | --- | --- |
| Session configuration | Direct | locale, timezone, viewport, headers, user agent, proxy, permissions |
| Cookies | Direct where policy/origin permits | preserve attributes and partition semantics; redact values from logs |
| Local/session storage | Direct or scripted import | origin scoped; version and size limits |
| Current URL/history checkpoint | Navigation/replay | do not fabricate inaccessible history entries |
| IndexedDB/cache/service-worker state | Capability-dependent | often unavailable initially; disclose partial transfer |
| DOM/JS heap | Not directly transferred | recreate through navigation and replay |
| Form state | Verified action replay or explicit field snapshot | avoid replaying secret values in logs |
| Action history | Deterministic replay | only idempotent/approved steps past checkpoint |

## Transfer bundle

A bundle contains metadata and encrypted payload sections:

- format and schema version;
- tenant/project/session/context IDs;
- source/destination engine versions;
- origin scopes;
- policy hash and expiry;
- state categories and omissions;
- current verified workflow checkpoint;
- action log with secret references;
- integrity hash and nonce.

## Migration algorithm

1. Pause new commands and establish a migration barrier.
2. Finish or cancel the active command deterministically.
3. Export allowlisted state and action checkpoint.
4. Allocate destination under equal or stronger isolation/network policy.
5. Apply configuration and state.
6. Navigate/replay to the checkpoint.
7. Verify URL, storage fingerprints, semantic/workflow postcondition, and authentication indicator where configured.
8. Commit destination as active engine.
9. Close source after a short rollback window, subject to resource policy.
10. Emit migration outcome and omissions.

## Side-effect safety

Actions are classified:

- `pure`: query/extraction/wait;
- `idempotent`: setting a field to known value;
- `conditionally-idempotent`: navigation, toggles, selections;
- `side-effecting`: submit, send, purchase, delete, publish.

Replay never repeats an unverified side-effecting action. Migration must occur before it, resume after a verified postcondition, or require human/workflow policy.

## Failure outcomes

- `STATE_EXPORT_UNSUPPORTED`
- `STATE_POLICY_DENIED`
- `DESTINATION_START_FAILED`
- `STATE_IMPORT_FAILED`
- `REPLAY_DIVERGED`
- `CHECKPOINT_VERIFICATION_FAILED`
- `MIGRATION_DEADLINE_EXCEEDED`

On failure, return source state if safely resumable, otherwise close with a classified error and reproduction evidence.
