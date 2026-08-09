# M0 exit waiver — Docker runtime

- Owner selection: option B
- Date: 2026-08-09
- Decision: proceed to M1 without Docker/Compose health and reset evidence.
- Recommendation: retain the waiver as a release limitation; install Docker and
  complete `just dev-up`, `just dev-health`, and reset rehearsal before beta.
- Evidence retained: M0 source gates, hosted fast-gates, and real two-worktree
  rehearsal passed. Docker Desktop provisioning failed at administrator/UAC.
- No security, production, or public compatibility claim is authorized by this
  waiver.
