---
title: "Event Loop and Runtime Scheduler"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define web task semantics, microtasks, timers, thread affinity, fairness, budgets, and cancellation."
---

# Event Loop and Runtime Scheduler

## Scope

This document covers the in-worker web event loop. Fleet/session placement is covered by `SCHEDULER_AND_ISOLATION.md`.

## Event-loop model

Each page/context has ordered task sources and a V8 microtask queue. The implementation must preserve required ordering while allowing bounded network and worker concurrency.

Conceptual sources:

- navigation/parser;
- DOM manipulation and user interaction;
- timers;
- networking/fetch;
- posted messages;
- workers/WebSockets;
- storage callbacks;
- command dispatch;
- lifecycle/cleanup.

After applicable tasks, drain microtasks with a budget. Infinite microtask production triggers a typed script/resource error rather than starving cancellation.

## Threading

- A V8 isolate and its bound DOM objects have one owning execution lane unless a supported V8 model explicitly permits otherwise.
- Network I/O and storage may run on asynchronous executors, returning completion messages to the owning loop.
- Cross-thread messages are typed, bounded, and cancellation-aware.
- No page callback executes while holding global scheduler or storage locks.

## Fairness

Use weighted scheduling across sessions and task classes. Interactive commands and cancellation receive priority, but no tenant/session can monopolize a worker. Track consumed CPU/time budget and yield between bounded units of work.

## Timers

Support monotonic scheduling, minimum delays/clamping where required, interval drift policy, cancellation, nesting behavior, and lifecycle suspension. Virtual time may be added for tests/workflows but is explicit.

## Deadlines

A command deadline yields a cancellation token propagated to waits, network, storage, and script termination. Session deadline is an upper bound. Cleanup has a separate bounded grace period.

## Resource budgets

Account and enforce:

- V8 heap and external memory;
- DOM nodes/attributes/text;
- network requests/bytes/connections;
- task and microtask execution time;
- event/listener counts;
- timers and pending promises;
- frames/workers/WebSockets;
- trace/artifact bytes.

Soft thresholds emit warnings/telemetry and may trigger fallback or policy action. Hard thresholds terminate the affected operation/session with a typed code.

## Deterministic test mode

Provide controlled clocks, seeded randomness hooks where permissible, recorded network fixtures, deterministic task checkpoints, and event sequence assertions. Do not alter production semantics merely to make tests deterministic; expose test-only configuration explicitly.
