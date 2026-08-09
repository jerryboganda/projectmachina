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
