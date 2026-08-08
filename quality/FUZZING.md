---
title: "Fuzzing and Sanitizer Program"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Continuously discover parser, DOM, protocol, network, storage, workflow, and FFI defects with bounded development smoke and long final campaigns."
---

# Fuzzing and Sanitizer Program

## High-value targets

- HTML tokenizer/tree builder and character references.
- CSS selector/XPath parsers.
- URL/header/cookie/parser and redirect handling.
- DOM mutation/event/custom-element/Shadow DOM sequences.
- JavaScript binding argument conversion and wrapper lifetime.
- C++ V8 bridge, snapshot loading, context teardown.
- Protocol frame/schema decoders.
- State-transfer bundle import and action replay.
- Workflow DSL/parser/validator.
- Archive/download/upload handling.
- Redaction and artifact serializers.

## Harness rules

No external Internet, deterministic seeds/time where feasible, strict memory/time limits, sanitizer builds, crash artifact with seed and build, and corpus minimization. Fuzzing must exercise production parsers/logic rather than a divergent toy path.

## Cadence

- Per relevant task: run regression corpus and a short bounded smoke for changed target.
- Scheduled: minutes-scale rotating targets.
- M8: hours-scale prioritized campaigns.
- M9: multi-hour/multi-day aggregate campaign on sanitizer variants.

## Sanitizers/tools

Use applicable address, undefined behavior, thread, memory, leak, and C++ sanitizers, plus Rust tools/lints and interpreter/model checking where useful. Some combinations require separate builds.

## Triage

Deduplicate by stack/root behavior, minimize input, classify severity/reachability, add regression, fix, rerun target and related corpus. A crash in unreachable debug-only code is still tracked, not silently discarded.

## Security handling

Potential exploitable findings follow restricted security issue and incident policy. Do not upload sensitive corpus or crash memory to public systems.
