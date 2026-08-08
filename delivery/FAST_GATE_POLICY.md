# Fast-gate policy

The protected `main` path requires the `fast-gate` workflow before merge. The
workflow is intentionally narrow enough for ordinary changes while preserving
the required safety checks:

- frozen pnpm installation and generated-contract drift;
- Rust format, compile, test, Clippy, and CMake bridge build;
- Svelte check/build and focused Node tests;
- architecture boundaries, redaction, supply-chain, threat traceability, and
  repository secret scanning;
- policy-negative fixtures for secret indicators and overlapping ownership
  scopes.

Heavy WPT, differential, fuzz, load, soak, chaos, penetration, and full
certification suites remain scheduled in M8/M9. Deferral is recorded, not
waived.

Untrusted pull-request jobs have read-only repository permissions and no
production, signing, or vault credentials. Hosted branch protection and
CODEOWNERS enforcement must be applied in GitHub settings before the first
trusted merge.
