---
title: "Project Machina — Agentic Development Documentation"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Master index for building the complete machine-native browser platform with one or more coding agents."
---

# Project Machina

Project Machina is the working codename for an independent, clean-room, machine-native browser and automation platform. It combines a high-throughput native engine with transparent Chromium fallback, a unified automation protocol layer, deterministic agent workflows, and a Svelte 5/SvelteKit control console.

This repository is a **development-control package**, not merely a design brief. It gives Claude Code, OpenAI Codex, GitHub Copilot, Google Antigravity, Google AI Studio agents, and human engineers the same source of truth, task graph, autonomy loop, quality gates, handoff protocol, and completion criteria.

> **Working-name notice:** “Project Machina” is a placeholder. Complete trademark and naming review before public launch.

## Recommended baseline

| Decision | Baseline |
| --- | --- |
| Browser architecture | Independent Rust native engine plus narrow C++ V8 bridge and Chromium fallback |
| Control-plane API | Unified typed command model exposed through HTTP, gRPC/events, CDP, WebDriver BiDi, MCP, and SDKs |
| Frontend | Svelte 5 + SvelteKit + TypeScript |
| Data services | PostgreSQL for durable state, Redis-compatible coordination, S3-compatible artifacts |
| Deployment | Docker Compose locally; Kubernetes for production |
| Agent concurrency | Two implementation agents in separate Git worktrees by default |
| Testing | Very small per-change gates; selected scheduled tests; one exhaustive final certification campaign |
| Licensing | Clean-room core with a permissive-license target, subject to dependency and legal review |

## Begin here

1. Read [`START_HERE.md`](START_HERE.md).
2. Accept or override defaults in [`OWNER_DECISIONS.md`](OWNER_DECISIONS.md).
3. Give your selected tool its matching entry file: [`AGENTS.md`](AGENTS.md), [`CLAUDE.md`](CLAUDE.md), [`CODEX.md`](CODEX.md), [`GEMINI.md`](GEMINI.md), [`ANTIGRAVITY.md`](ANTIGRAVITY.md), or [`AI_STUDIO.md`](AI_STUDIO.md).
4. Start milestone M0 from [`planning/MASTER_TASK_GRAPH.md`](planning/MASTER_TASK_GRAPH.md).
5. Keep [`agents/CURRENT_STATE.md`](agents/CURRENT_STATE.md), [`agents/WORK_QUEUE.md`](agents/WORK_QUEUE.md), and [`agents/BLOCKERS.md`](agents/BLOCKERS.md) current after every task.

## Local development bootstrap

The working tree may be developed locally before its first commit. From the
repository root, install the pinned toolchains and run:

```text
pnpm bootstrap
pnpm contract:generate
pnpm contract:check
pnpm security:check
pnpm test
```

Use `just doctor-strict` before committing toolchain-sensitive changes; it
enforces the exact Rust, Node, CMake, Clang, Ninja, and Buf versions recorded
under `toolchains/versions.toml`.

The first commit should contain the complete bootstrap diff and be pushed to the
protected `main` branch through the repository review policy. Do not add secrets,
raw traces, build output, or claim files to the commit.

## Documentation map

| Area | Contents |
| --- | --- |
| `agents/` | Autonomous loop, multi-agent ownership, role prompts, state, recovery, handoffs, and approval rules |
| `.agents/skills/` | Cross-tool executable skills for implementation, review, benchmarking, and final certification |
| `.claude/agents/` | Claude Code specialist subagents |
| `.github/agents/` and `.github/prompts/` | GitHub Copilot custom agents and reusable prompt files |
| `antigravity/` | Antigravity personas, skills catalog, and autonomous workflow controller |
| `product/` | Product charter, PRD, requirements, personas, scope, metrics, roadmap, and risks |
| `architecture/` | System, native engine, fallback, command bus, frontend, control plane, data, isolation, errors, and ADRs |
| `protocols/` | HTTP, gRPC/events, CDP, WebDriver BiDi, MCP, SDK contracts, compatibility, and versioning |
| `security/` | Threat model, sandboxing, tenant isolation, egress, secrets, privacy, supply chain, and incident response |
| `quality/` | Fast inner loop, final heavy campaign, WPT, differential tests, fuzzing, performance, and acceptance |
| `delivery/` | Environments, builds, CI/CD, branches, release, deployment, licensing, SLOs, and runbooks |
| `operations/` | On-call, capacity, backup, disaster recovery, retention, and troubleshooting |
| `planning/` | Definitions, dependency map, 121-task implementation graph, and milestone task packets |
| `research/` | Lightpanda gap analysis, technology choices, tool interoperability, testing rationale, and sources |

## Source-of-truth order

When instructions conflict, agents apply this order:

1. Security and legal constraints.
2. Accepted architecture decision records.
3. Product requirements and acceptance criteria.
4. Protocol and data contracts.
5. Milestone and task packet.
6. Tool-specific instructions.
7. An agent's local plan.

An agent must not silently resolve a conflict by inventing behavior. It records the conflict in `agents/BLOCKERS.md`, applies the safest reversible interpretation, and continues only where unaffected.

## Autonomous completion model

No current coding tool can honestly guarantee uninterrupted, unattended completion of a program of this size. Sessions end, quotas change, credentials expire, and some decisions require a human accountable owner. This pack instead provides **checkpointed autonomy**:

- every task is independently resumable;
- state and evidence live in the repository;
- agents claim non-overlapping file scopes;
- failures have bounded repair loops;
- another supported tool can resume from the same state;
- only explicit human gates stop the queue;
- the loop continues until every required task and final acceptance gate is complete.

## Testing model

The project optimizes for development speed without postponing all feedback until the end. Each task gets only the smallest fast gate needed to catch syntax, compilation, contract, and obvious behavioral failures. Long WPT, broad differential, sustained load, chaos, penetration, and soak campaigns are consolidated into milestone windows and the final certification campaign. See [`quality/TEST_STRATEGY.md`](quality/TEST_STRATEGY.md).

## Definition of project complete

The project is complete only when:

- all required tasks in M0–M9 are complete or formally waived;
- all protocol and capability claims have executable evidence;
- native and fallback paths meet the published success targets;
- the final heavy test campaign passes its release gates;
- security, licensing, operations, disaster recovery, and release documentation are approved;
- a reproducible build and rollback exist;
- public performance claims are supported by a fair benchmark.
