---
name: final-certification
description: Execute the M9 frozen-candidate certification campaign and prepare release evidence without self-authorizing GA.
---

# Final certification

1. Verify M0–M8 completion, critical blocker closure, feature/schema/dependency freeze, and candidate digest.
2. Verify reproducible build, SBOM, signatures, provenance, migrations, and rollback assets.
3. Execute the ordered suites in `quality/FINAL_HEAVY_TEST_CAMPAIGN.md`.
4. Preserve raw artifacts and link every release requirement to evidence.
5. For a failure, determine evidence invalidation scope, open a bounded repair, build a new candidate digest, and rerun the affected dependency closure.
6. Record every waiver with owner, impact, capability/claim removal, expiry, and follow-up.
7. Rehearse deployment, rollback, backup/restore, and disaster recovery.
8. Produce the final release evidence index and stop at the human GA approval gate.

Never reuse evidence whose inputs no longer match the candidate hash.
