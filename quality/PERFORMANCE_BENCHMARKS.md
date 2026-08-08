---
title: "Performance Benchmark Plan"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Measure fair end-to-end performance, resource efficiency, success, and fallback against reference implementations."
---

# Performance Benchmark Plan

## Primary rule

Compare equivalent workloads, fidelity, isolation, wait condition, network, success criteria, and verification. A failed or incomplete task cannot improve throughput.

## Systems

- Project Machina native mode.
- Project Machina automatic hybrid mode.
- Project Machina Chromium-only mode.
- Lightpanda build/version where license and environment permit evaluation.
- Headless Chromium directly.
- Firefox headless where useful.

## Workload groups

1. Static/server-rendered extraction.
2. JavaScript-heavy pages/SPAs.
3. Login/forms/storage/navigation.
4. Multi-step semantic agent tasks.
5. Protocol/client automation suites.
6. High-concurrency multi-tenant sessions.
7. Long-lived workflows and repeated replay.

## Measures

- verified success and failure classification;
- cold/warm session startup;
- p50/p95/p99 end-to-end latency;
- CPU time and core-hours;
- average/peak/steady memory and GB-hours;
- requests/bytes/connections;
- fallback and migration latency/cost;
- retries and timeout;
- tasks/core-hour and tasks/GB-hour;
- cost per 1,000 verified tasks;
- LLM token use for discovery/recovery and zero-token replay rate.

## Controls

Same hardware/node class, CPU pinning/noise notes, memory limits, OS/kernel, container/isolation topology, engine/client versions, concurrency, cache state, DNS/proxy, network shaping, page fixtures or timestamped corpus, warm-up, repetitions, statistical intervals.

## Public report

Publish methodology, raw/aggregated artifacts, scripts, failures, exclusions, and limitations. Separate project-reported competitor results from independently reproduced results. Public claims require independent approval gate.

## Regression budgets

Define per-workload tolerances after baseline. Block release for material unexplained regression in verified throughput, memory, startup, or tail latency, especially if hidden by more fallback/retries.
