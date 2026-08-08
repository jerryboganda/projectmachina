---
title: "Autonomous Continuous Development Loop"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Specify a deterministic, resumable loop that supported coding agents can execute until the project is complete."
---

# Autonomous Continuous Development Loop

## Design principle

Continuity belongs to repository state and the orchestrator, not to a single model context. Every iteration is bounded, evidence-producing, and safe to resume.

## State machine

```text
BOOT
  -> RECONCILE
  -> SELECT_TASK
  -> VERIFY_DEPENDENCIES
  -> CLAIM_TASK_AND_PATHS
  -> PREPARE_WORKTREE
  -> LOAD_CONTEXT
  -> PLAN
  -> IMPLEMENT
  -> FAST_GATE
  -> SELF_REVIEW
  -> INDEPENDENT_REVIEW
  -> REPAIR (maximum 3 cycles)
  -> READY_TO_MERGE
  -> MERGE_QUEUE
  -> UPDATE_EVIDENCE_AND_STATE
  -> RELEASE_CLAIMS
  -> SELECT_TASK

Any state -> HANDOFF when session/resource limit approaches
Any state -> BLOCKED when a hard stop condition is proven
All required tasks complete -> FINAL_CERTIFICATION
Final certification passed + approvals -> COMPLETE
```

## Executable pseudocode

```text
while project.status != COMPLETE:
    facts = inspect_git_pr_ci_worktrees_claims()
    state = reconcile(facts, durable_state)

    if session_budget_low():
        write_handoff(state)
        exit_resumable()

    task = highest_priority_ready_task(state)

    if task is None:
        if all_required_tasks_complete(state):
            run_or_resume_final_certification()
        else:
            record_global_or_external_blockers()
            attempt_unaffected_repair_or_research_tasks()
        continue

    if not atomically_claim(task, task.write_scope):
        continue

    worktree = prepare_isolated_worktree(task)
    context = load_minimum_required_context(task)
    plan = produce_bounded_plan(task, context)

    for repair_cycle in 0..3:
        implement(task, plan, worktree)
        result = run_fast_gate(task)
        review = self_review_and_independent_review(task)

        if result.passed and review.blocking_findings == 0:
            enqueue_merge(task)
            merge_in_dependency_order(task)
            persist_evidence_and_state(task)
            release_claims(task)
            break

        if repair_cycle == 3:
            mark_blocked(task, evidence=result + review)
            release_claims(task)
```

## Step contract

### Reconcile

Never trust `CURRENT_STATE.md` blindly. Detect orphaned worktrees, merged branches still marked active, expired claims, failed CI, and PRs missing state updates.

### Select

Select work that is ready now. Do not ask the owner which task to do when the task graph already determines the answer.

### Claim

A claim contains task ID, agent ID, branch, worktree, path globs, timestamp, lease expiry, and heartbeat. Claim task plus paths atomically. See `MULTI_AGENT_CONCURRENCY.md`.

### Load context

Read the task packet and only directly relevant contracts. Use repository search rather than loading the entire documentation pack into the model context. Record any ambiguity before coding.

### Plan

Plans should be short and implementation-oriented:

- files to create/change;
- interfaces affected;
- invariants;
- fast-gate commands;
- migration/rollback impact;
- risks.

### Implement

Prefer one coherent vertical slice. Do not add speculative features outside acceptance criteria. Update public docs, schemas, generated bindings, and telemetry in the same task when required.

### Fast gate

Use the smallest relevant gate. Failure blocks merge but does not trigger the exhaustive suite. See `quality/FAST_INNER_LOOP.md`.

### Review

The independent reviewer checks behavior, architecture, security, compatibility, test adequacy, and diff scope. The reviewer does not rewrite the implementation unless assigned a separate repair task.

### Repair

Each repair cycle must target concrete findings. After three cycles, record a blocker with reproduction evidence and continue independent ready work.

### Merge and state

The merge is not complete until task status, capability evidence, decision log, deferred-test inventory, and next-task readiness are updated.

## Heartbeats and leases

- Default task lease: 90 minutes.
- Heartbeat: every 10 minutes during active tool execution or after each material step.
- Grace period: 20 minutes.
- An orchestrator may reclaim only after lease plus grace, no active process evidence, and a recovery inspection.

Long-running compilation or tests must write command, PID/job ID, start time, deadline, and output path rather than extending a lease silently.

## Automatic continuation policy

After a successful merge, immediately select the next ready task. Do not stop to ask whether to continue. Pause only under `HUMAN_APPROVALS.md` or a hard blocker.

## Bounded autonomy

Autonomy is constrained by:

- task scope;
- file ownership;
- command allow/deny policy;
- resource ceilings;
- repair limits;
- human gates;
- security invariants;
- acceptance evidence.

This prevents “continuous” from becoming uncontrolled.
