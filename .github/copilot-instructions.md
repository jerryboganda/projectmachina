---
applyTo: "**"
---

# Project Machina repository instructions

Read `/AGENTS.md` first. It is authoritative. Then read `/OWNER_DECISIONS.md`, `/agents/CURRENT_STATE.md`, `/agents/WORK_QUEUE.md`, and the assigned task packet.

## Mission

Build the independent Rust/V8 machine-native browser, automatic Chromium fallback, unified protocol adapters, deterministic workflows, and SvelteKit console defined in this repository. Work through the task graph with durable evidence and resumable handoffs.

## Before editing

1. Verify the task is ready and not already claimed.
2. Record the claim, branch, worktree, base commit, and allowed write paths.
3. Read the direct architecture/protocol/security documents linked from the task.
4. Do not edit outside the claimed scope.
5. Do not change public contracts or ADR decisions implicitly.

## Implementation rules

- One internal typed command bus owns browser semantics.
- Native Rust engine and Chromium fallback must report explicit engine/capability state.
- No silent unsupported operations.
- Keep the C++ V8 bridge narrow; document ownership and all unsafe invariants.
- Redact secrets and page-sensitive content from logs.
- Use cancellation, deadlines, budgets, and typed errors.
- Svelte 5/SvelteKit/TypeScript is the frontend baseline.
- Generate adapters and SDKs from canonical schemas; never hand-edit generated output.
- Do not copy Lightpanda implementation details into this clean-room project.

## Validation

Run only the task's fast gate: formatting/static validation, changed-package compilation/type checking, changed unit/contract tests, a focused smoke path, and immediate high-risk security checks. Do not run the full WPT, corpus, load, fuzz, chaos, or soak suites on every change; those are scheduled in M8/M9.

## Completion

A response or pull request must identify task ID, changed files, commands/results, acceptance evidence, deferred heavy tests, risks, and next ready task. Update durable state before stopping. Use an independent custom agent for review on critical work.
