---
title: "M3 Entry-Readiness and Dependency-Wave Analysis"
project: "Project Machina"
document_status: "design-draft"
version: "1.0.0"
owners: "Architecture"
purpose: "Pre-compute M3's real dependency-wave graph and M2-gating status so M3 can start immediately once M2 exits, mirroring the exercise already applied to M2 in agents/WORK_QUEUE.md."
---

# M3 Entry-Readiness and Dependency-Wave Analysis

## Scope and method

Source of task scope/dependencies/acceptance criteria:
`planning/MILESTONE_03_NATIVE_WEB_APIS_AND_AUTOMATION.md` (M3-T01 through M3-T15).
Format mirrors the real dependency graph already built for M2 in
`agents/WORK_QUEUE.md` § Ready: wave = earliest point a task can honestly
start, rank = suggested execution order, concurrency capped at two active
implementation agents per `AGENTS.md` and the M3 milestone doc's own
scheduling note.

This document is planning only. No crate was modified, no implementation
work was started, and no milestone source file was edited (per
`agents/WORK_QUEUE.md`'s "Queue mutation rules": task IDs/acceptance criteria
change only through the milestone source, not this derived view).

## 1. Current M2 gating status (input fact, reconciled against Git)

Per `agents/WORK_QUEUE.md` (last reconciled 2026-08-09): 8 of 14 M2 tasks are
merged (T01/T02/T03/T04/T05/T10/T11/T13). Five remain open and are the
critical path: **M2-T06 → M2-T07 → M2-T08 → {M2-T09, M2-T12} → M2-T14**
(M2-T06 is not yet started even though its toolchain and design are ready;
every downstream M2 task is blocked on it). M1-T09 (capability registry v0)
is already merged, so it is not a live blocker for anything below despite
being listed as a formal dependency of M2-T14 and M3-T10.

**Conclusion: zero M3 tasks can honestly begin real implementation today.**
Every M3 task's stated dependency set resolves, directly or transitively,
to at least one of the five open M2 tasks. This confirms the task's
prohibition on starting M3 implementation now is not just a policy
formality — it is also the honest state of the dependency graph.

## 2. Per-M3-task M2 gating (direct and transitive)

| M3 task | Directly stated M2 deps | Minimal M2 task(s) that must merge first | Notes |
| --- | --- | --- | --- |
| M3-T01 | M2-T11 (merged), M2-T12 (open) | M2-T12 | M2-T12 itself needs M2-T05(merged)+M2-T08, so this transitively needs T06/T07/T08 too. |
| M3-T02 | M2-T10 (merged), M2-T11 (merged) + M3-T01 | M2-T12 (via M3-T01) | Pure M3-internal fan-out otherwise. |
| M3-T03 | M2-T09 (open), M2-T10 (merged), M2-T13 (merged) | M2-T09 | Does **not** need M2-T12. Second-earliest M3 task after the M2-T07/T08 pair below. |
| M3-T04 | M2-T05 (merged), M2-T07 (open), M2-T08 (open), M2-T11 (merged) | M2-T07, M2-T08 | Does **not** need M2-T09 or M2-T12 — earliest-unlockable M3 task alongside M3-T07. |
| M3-T05 | M2-T09 (open), M2-T12 (open) + M3-T04 | M2-T09 **and** M2-T12 | Needs both open M2 tasks directly, not just via M3-T04. |
| M3-T06 | M2-T02 (merged), M2-T12 (open) | M2-T12 | Also inherits `BLK-004`'s deferred scope — see § 4. |
| M3-T07 | M2-T02 (merged), M2-T08 (open), M2-T07 (open) | M2-T07, M2-T08 | Does **not** need M2-T09 or M2-T12 — same tier as M3-T04. |
| M3-T08 | M2-T06 (open), M2-T08 (open), M2-T12 (open) | M2-T06, M2-T08, M2-T12 | Needs the full chain including M2-T12. |
| M3-T09 | M3-T08, M3-T06 (both M3-internal) | M2-T12 (transitively, via both parents) | |
| M3-T10 | M3-T06, M1-T09 (merged) | M2-T12 (via M3-T06) | M1-T09 dependency is already satisfied. |
| M3-T11 | M3-T01, M3-T05, M3-T06 | M2-T09 **and** M2-T12 (via M3-T05) | |
| M3-T12 | M2-T13 (merged), M3-T04, M3-T05 | M2-T09 **and** M2-T12 (via M3-T05) | |
| M3-T13 | M3-T12, M2-T13 (merged) | M2-T09 **and** M2-T12 (via M3-T12) | |
| M3-T14 | M3-T02, M3-T03, M3-T05, M3-T10 | M2-T09 **and** M2-T12 | Union of its parents' requirements. |
| M3-T15 | M3-T01…M3-T14, M2-T14 | M2-T14 directly (the formal M2 exit gate) | Final coverage gate; explicitly "no parallel." |

Reading this against M2's own wave order (`agents/WORK_QUEUE.md`:
M2-T06→wave2, M2-T07→wave3, M2-T08→wave4, {M2-T09, M2-T12}→wave5,
M2-T14→wave7) gives a **staggered, not all-or-nothing**, M3 unlock sequence:

- As soon as **M2-T08** merges (M2 wave 4): M3-T04 and M3-T07 become
  dependency-ready (still gated by whatever else the milestone's entry
  criteria require — see § 5).
- As soon as **M2-T09** merges (M2 wave 5): M3-T03 additionally becomes
  ready.
- As soon as **M2-T12** merges (also M2 wave 5): M3-T01, M3-T06, M3-T08
  additionally become ready.
- Once both M2-T09 and M2-T12 are merged, the rest of the M3-internal graph
  (M3-T02, T05, T09, T10, T11, T12, T13, T14) opens up per § 3 below.
- **M2-T14** (formal M2 exit) is required only for M3-T15, the final M3
  coverage gate — not for any earlier M3 task's stated dependencies.

This is a real scheduling opportunity (M2-T09/T12 land two waves before
M2-T14 in M2's own graph) but it is a **policy question, not a decision this
document makes**: the M3 milestone's entry criteria say "M2 native
fundamentals pass," which the repo's own precedent
(`M2-ENTRY-WAIVER-M1-EXIT` in `agents/WAIVERS.md`) shows must be treated as
requiring an explicit recorded waiver, not silent early start. If the
orchestrator wants to claim M3-T04/T07 (or later M3-T01/T03/T06/T08) before
M2-T14 formally merges, that requires an owner-recorded
`M3-ENTRY-WAIVER-*`-style waiver analogous to the existing one, not an
autonomous decision by an implementing agent.

## 3. M3-internal wave graph (assuming all required M2 dependencies are met)

This mirrors the M2 table's structure: wave = earliest start once every
listed M2 dependency for that task is satisfied; rank = suggested order
honoring the two-concurrent-agent cap; lane = suggested execution track.

| Rank | Task | Wave | Role | M3-internal deps | Additional M2 gate (§2) | Suggested lane |
| ---: | --- | ---: | --- | --- | --- | --- |
| 1 | M3-T04 | 1 | native-engine | — | M2-T07, M2-T08 | A |
| 2 | M3-T07 | 1 | native-engine | — | M2-T07, M2-T08 | B |
| 3 | M3-T01 | 1 | native-engine | — | M2-T12 | A (after T04) |
| 4 | M3-T03 | 1 | native-engine | — | M2-T09 | B (after T07) |
| 5 | M3-T06 | 1 | native-engine + security | — | M2-T12; **inherits BLK-004, see §4** | A |
| 6 | M3-T08 | 1 | native-engine | — | M2-T06, M2-T08, M2-T12 | B |
| 7 | M3-T02 | 2 | native-engine + agent-runtime | M3-T01 | (via T01) M2-T12 | A |
| 8 | M3-T05 | 2 | native-engine | M3-T04 | M2-T09, M2-T12 (direct) | B |
| 9 | M3-T09 | 2 | native-engine | M3-T08, M3-T06 | (via parents) M2-T12 | A |
| 10 | M3-T10 | 2 | native-engine + protocol | M3-T06 | (via T06) M2-T12 | B |
| 11 | M3-T11 | 3 | native-engine + security | M3-T01, M3-T05, M3-T06 | M2-T09, M2-T12 | A |
| 12 | M3-T12 | 3 | native-engine | M3-T04, M3-T05 | M2-T09, M2-T12 | B |
| 13 | M3-T14 | 3 | native-engine + platform | M3-T02, M3-T03, M3-T05, M3-T10 | M2-T09, M2-T12 | A (after T11) |
| 14 | M3-T13 | 4 | native-engine + agent-runtime | M3-T12 | M2-T09, M2-T12 | A |
| 15 | M3-T15 | 5 | orchestrator + quality | M3-T01…M3-T14 | **M2-T14 (formal M2 exit)** | A (no parallel) |

Wave-1 has six dependency-ready tasks with only a two-agent cap, so — same
as M2's queue — expect wave 1 to actually take three scheduling rounds
(e.g. {T04,T07} → {T01,T03} → {T06,T08}) even though all six share the same
"wave" label in the earliest-start sense. `M3-T05` and `M3-T06` both carry
`critical` risk and both touch network/security semantics; do not schedule
them in the same lane simultaneously with anything else that reads/writes
`crates/network` until M3-T06 lands, to avoid the same kind of contract race
this repo has already had to correct once for M2-T05 (see the
"Process correction" note in `agents/WORK_QUEUE.md`).

`M3-T15` is explicitly "no parallel," matches M2-T14's treatment, and is
gated on the formal M2 exit criteria in
`planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md`'s own exit-criteria
section, not merely on the M3 tasks above it.

## 4. BLK-004 explicitly flagged into M3-T06

`agents/BLOCKERS.md` BLK-004 records that M2-T02's merged network loader
deferred the following items explicitly to M3-T06, rather than silently
dropping them:

1. full cookie-jar / CORS-credential-aware redirect forwarding,
2. referrer-policy computation across redirects,
3. HTTP caching semantics,
4. HTTP/2 server push and HTTP/3,
5. proxy support (CONNECT/SOCKS),
6. a DNS-rebinding-hardened resolver/cache with tenancy-aware variants,
7. certificate pinning/strict OCSP policy,
8. connection pooling/concurrency caps (flagged as a separate
   pre-production follow-up, not strictly a security gap).

Cross-checking this list against M3-T06's acceptance criteria as written in
`planning/MILESTONE_03_NATIVE_WEB_APIS_AND_AUTOMATION.md`:

- **Covered explicitly:** items 1–3 ("Implement prioritized CORS/preflight,
  referrer policy, redirect credential/header behavior and cache
  semantics"; "Priority fetch/CORS/referrer/cookie tests pass"; "Cache/
  redirect cannot bypass network policy or leak tenant identity" — this
  last clause also covers the tenancy-aware half of item 6).
- **Not explicitly named in M3-T06's written acceptance criteria (gap):**
  - Item 4 (HTTP/2 server push, HTTP/3) — no mention anywhere in M3-T06's
    deliverables/acceptance text.
  - Item 5 (proxy CONNECT/SOCKS support) — M3-T10's deliverables instead
    say "Support scoped proxy references/auth and DNS mode," which may be
    intended to absorb this item, but M3-T06 and M3-T10 do not
    cross-reference each other, so there is a real risk this item is
    silently dropped between the two tasks (each implementer could
    reasonably assume the other owns it).
  - Item 6's non-tenancy half (a genuinely DNS-rebinding-hardened
    resolver/cache beyond what M2-T02 already shipped) — not named.
  - Item 7 (certificate pinning / strict OCSP policy) — not named.
  - Item 8 (connection pooling/concurrency caps) — explicitly called a
    separate pre-production follow-up in BLK-004 itself, so its omission
    from M3-T06 is intentional and correctly scoped, not a gap.

**Recommendation (not applied by this document):** before M3-T06 is
claimed, the architect/owner should either (a) amend
`planning/MILESTONE_03_NATIVE_WEB_APIS_AND_AUTOMATION.md`'s M3-T06 and
M3-T10 deliverables/acceptance criteria to explicitly enumerate BLK-004
items 4, 5, 6, 7 and assign definitive ownership (M3-T06 vs. M3-T10 vs. a
new deferred task), or (b) leave the milestone text as-is but require
M3-T06's completion evidence to explicitly reconcile every BLK-004 item —
accept, implement, or re-defer with a new blocker record — rather than
letting any of them lapse silently. Per `agents/WORK_QUEUE.md`'s queue
mutation rules, this document does not itself edit the milestone source;
it only flags the gap for the task owner.

## 5. M2→M3 handoff / exit-criteria cross-check

`planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md`'s exit criteria
section (verified present) and M3's own entry criteria
("M2 native fundamentals pass"; "Capability registry and router can enable
native features incrementally") both point at the same gate: **M2-T14**
("Reach the target native corpus gate"), which is the only M2 task that
certifies the milestone's coverage/quality bar rather than an individual
capability. M1-T09 (capability registry/router) is already merged and
independently satisfies the second M3 entry criterion today. No other
explicit M2→M3 handoff artifact (e.g., a dedicated handoff document) exists
beyond the milestone's own entry/exit-criteria text; this document is the
requested substitute pre-computation.

## 6. What is genuinely M3-only prep that could start now

Per the task's explicit prohibition, no implementation code should start.
The pattern this repo already used ahead of M2-T06 (design/security/
research agents producing implementation-ready specs before the task's
dependencies cleared — see `agents/CURRENT_STATE.md`'s note on the
wave-1 M2 design pack merged via #27) is a reasonable analog for M3.
Non-implementation prep that could start immediately, without touching any
crate, and is not blocked on any M2 task:

- Design/spec documents (in `.agent-state/design/`) for the earliest-tier
  M3 tasks — M3-T04 (Shadow DOM/custom elements), M3-T07 (WebSocket), M3-T01
  (forms), M3-T06 (network hardening, informed by § 4 above), and M3-T08
  (workers) — so implementation can begin the moment each task's M2
  dependency lands, mirroring the M2-T06 precedent.
- WPT/fixture-corpus selection and target-coverage definition for M3-T15,
  since that task's acceptance bar ("native fast path reaches the M3 agreed
  target on stable selected corpus") is currently undefined in the
  milestone doc and will otherwise block M3-T15 even after everything else
  merges.
- Reconciling the BLK-004/M3-T06/M3-T10 scope-ownership gap identified in
  § 4, which is a documentation fix, not implementation.
- Confirming capability-matrix entries the M3 tasks will need to populate
  (per each task's "update the capability matrix when applicable"
  completion-evidence requirement) exist and are correctly named ahead of
  time.

None of this was performed by this document — it is scoped here as
recommended follow-on prep work, consistent with the "planning only"
instruction for this task.
