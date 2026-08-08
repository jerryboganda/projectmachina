---
title: "Canonical Agent Instructions"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define tool-neutral rules that every autonomous coding agent must follow."
---

# Canonical Agent Instructions

These instructions apply to every coding agent, subagent, reviewer, and orchestration tool working on Project Machina. Tool-specific files may add mechanics but may not weaken these rules.

## Mission

Build the complete Project Machina platform described by the product, architecture, protocol, security, quality, delivery, operations, and planning documentation. Continue through the task graph until all required work and final certification are complete, pausing only for an explicit human gate or a proven external blocker.

## Mandatory reading order

1. `START_HERE.md`
2. `OWNER_DECISIONS.md`
3. `agents/CURRENT_STATE.md`
4. `agents/WORK_QUEUE.md`
5. `agents/AUTONOMOUS_LOOP.md`
6. `agents/MULTI_AGENT_CONCURRENCY.md`
7. `planning/MASTER_TASK_GRAPH.md`
8. The selected milestone file and task packet
9. Relevant architecture, protocol, security, and quality documents
10. Accepted ADRs

## Non-negotiable architecture

- Independent clean-room implementation unless the owner changes D01.
- Rust native engine with a narrow C++ V8 boundary.
- Chromium compatibility fallback behind the same typed command model.
- One internal command bus; adapters must not implement divergent browser behavior.
- Svelte 5/SvelteKit/TypeScript for the web console unless D04 changes.
- Explicit capability detection and typed errors; no silent unsupported operations.
- Durable state and resumable sessions.
- Multi-tenant isolation selected by policy tier.
- Security controls may not be deferred merely for speed.

## Autonomous task loop

For each task:

1. Reconcile durable state with Git, worktrees, pull requests, and CI.
2. Select the highest-priority ready task whose dependencies are complete.
3. Atomically claim the task and its file ownership scope.
4. Create or reuse a dedicated branch and worktree.
5. Read only the context needed for the task, then write a short implementation plan.
6. Implement the smallest coherent change satisfying all acceptance criteria.
7. Run the fast gate defined by the task and `quality/FAST_INNER_LOOP.md`.
8. Self-review the diff for correctness, security, compatibility, and unnecessary scope.
9. Request independent review; repair at most three bounded cycles.
10. Merge through the queue, update state and capability evidence, release ownership, and immediately select the next task.

See `agents/AUTONOMOUS_LOOP.md` for the executable state machine.

## Multi-agent rules

- Default maximum: two concurrent implementation agents.
- Each agent uses a separate Git worktree and branch.
- File ownership must not overlap. Read access may overlap.
- Shared interfaces are changed by one designated contract owner first.
- Generated files are owned by the task that owns their source definition.
- Never force-push another agent's branch, delete another worktree, or rewrite shared history.
- Communicate through task state and handoff records, not assumed chat memory.

## Quality strategy

The owner prefers heavy tests near the end. Honor that by keeping development gates narrow, not by eliminating feedback:

- always format and compile/type-check changed packages;
- run changed-unit and contract tests;
- run one focused smoke path for behavior changes;
- run security-specific checks immediately for parser, sandbox, auth, secrets, networking, and unsafe code;
- defer broad WPT, corpus differential, long fuzz, load, chaos, and soak suites to scheduled windows and M9.

A task cannot be marked complete when its changed code does not compile or its contract evidence is missing.

## Coding standards

- Prefer simple, explicit, typed interfaces.
- No `unwrap`, unchecked cast, or unsafe block in production Rust without documented invariant and focused test.
- Keep the C++ boundary minimal and ownership rules documented.
- Use structured errors and cancellation-aware asynchronous code.
- Redact secrets and page-sensitive data from logs by default.
- Feature flags must have an owner, removal condition, and default behavior.
- All user-visible or public protocol behavior requires documentation and compatibility tests.
- Avoid premature abstraction, but never duplicate protocol semantics across adapters.

## Decision rules

An agent may decide without asking when the choice is:

- reversible;
- within an accepted ADR and task scope;
- not security-, legal-, privacy-, or budget-sensitive;
- unlikely to create a public compatibility commitment.

Record consequential decisions in the task log. Propose a new ADR when a decision changes component boundaries, data durability, public protocol semantics, isolation, security posture, or core technology.

## Stop conditions

Stop only the affected workstream when:

- a human-required decision is reached;
- credentials or an external service are unavailable;
- legal/license review is required;
- three repair cycles fail for the same acceptance condition;
- repository state cannot be reconciled safely;
- continuing would weaken a required control;
- the task graph is complete and M9 certification passes.

Do not stop merely because the work is large, unfamiliar, or difficult. Record the blocker and continue independent ready work.

## Required completion report

Every task completion must include:

- task ID and title;
- branch, commit, and pull request;
- changed files and owned scope;
- decisions and assumptions;
- commands run and exact results;
- acceptance-criterion evidence;
- deferred heavy tests;
- capability/security/documentation updates;
- known risks and next ready task.
