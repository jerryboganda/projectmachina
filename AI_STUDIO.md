---
title: "Google AI Studio Managed-Agent Configuration"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define a safe, resumable configuration for running Project Machina tasks in Google AI Studio agent environments."
---

# Google AI Studio Managed-Agent Configuration

This document supplements `GEMINI.md` for managed-agent environments.

## Environment template

Use an immutable base image or reproducible setup script with pinned versions. Mount one task worktree at a time. Attach only the secrets required by that task and make production credentials unavailable by default.

Recommended environment capabilities:

| Capability | Default |
| --- | --- |
| Repository read/write | Current worktree only |
| Internet | Allowlisted package registries and documentation |
| Docker | Rootless or isolated runner |
| Cloud control plane | Disabled during ordinary coding |
| Artifact upload | Enabled to task-specific prefix |
| Long-running commands | Allowed with explicit deadline and heartbeat |
| Human oversight | Required only at listed approval gates |

## Run contract

Every managed run receives a task packet containing:

- immutable task ID;
- base commit;
- branch and worktree path;
- dependency state;
- allowed write paths;
- acceptance criteria;
- fast-gate commands;
- resource ceiling;
- required handoff destination.

The run returns a patch or commit, evidence manifest, unresolved findings, and continuation recommendation.

## Continuous execution

An external orchestrator, not an individual model context, owns continuity. It reads durable task state, launches the next bounded run, validates the returned evidence, and advances the task state machine. This design tolerates agent restarts and avoids treating one long conversation as infrastructure.
