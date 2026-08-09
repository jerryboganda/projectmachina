# Active waivers

## M0-EXIT-WAIVER-DOCKER — owner-selected option B

- Owner decision: proceed to M1 without Docker/Compose runtime health/reset evidence.
- Selected: 2026-08-09
- Scope: M0-T11 container health and the container-dependent portion of M0-T12.
- Evidence retained: source gates, hosted fast-gate, and real Git/worktree rehearsal
  passed; Docker Desktop installation was attempted and failed at administrator/UAC.
- Constraint: do not claim Docker production readiness, disaster recovery, or
  full local-stack certification from this waiver.
- Revisit: before beta/RC/GA and before any production container claim.

## M2-ENTRY-WAIVER-M1-EXIT — owner-selected early start

- Owner decision: begin M2 native-engine source implementation before M1 is
  formally declared exited.
- Selected: 2026-08-09
- Rationale: M1's remaining exit gap (`BLK-003`) is specifically the Chromium
  track — no real Chromium process launch/CDP code exists yet in
  `crates/chromium-adapter`. The native engine (M2) is an architecturally
  independent track from the Chromium adapter; both implement the same
  `EngineAdapter`/command-bus contract but do not share code. Starting M2
  source work does not depend on the Chromium launch implementation landing
  first.
- Scope: M2-T01 through M2-T14 source implementation only.
- Constraint: `M2-T14` (native worker integration + first native corpus gate)
  and any M2 exit claim still require real, non-simulated evidence — no
  fabricated WPT/corpus pass results. `M1` is still not exited; do not claim
  M1 exit, production readiness, or a certified compatibility baseline from
  this waiver. `BLK-003` remains open and must still be resolved before M1
  exit or before any Chromium-backed (non-native) production claim.
- Concurrency: the milestone-doc cap of "at most two implementation tasks
  concurrently" is honored for actual crate-writing (builder) agents;
  additional concurrent reviewer/security/architect/research agents are not
  counted against that cap.
- Revisit: before M2 exit is declared, and before M1 exit is declared.
