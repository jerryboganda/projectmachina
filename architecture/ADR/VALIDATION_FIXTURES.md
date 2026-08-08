# Architecture boundary fixtures

`scripts/architecture/check-boundaries.test.mjs` contains a deliberate
protocol-to-engine import violation and an allowed inward-only import. The
negative fixture must fail with an actionable rule identifier; the positive
fixture must remain clean.
