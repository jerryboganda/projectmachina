# Clean-room contribution controls

Project Machina follows the selected independent clean-room implementation
strategy. Contributors and coding agents must:

- implement behavior from public standards, approved Project Machina contracts,
  and independently authored tests;
- avoid copying implementation details, source code, non-public artifacts, or
  proprietary test data from unrelated browser engines;
- record third-party source, version, purpose, license, and integrity evidence
  for every dependency;
- keep generated output tied to its canonical source definition;
- escalate a suspected source-contamination or license issue through
  `H-LEGAL-01` before merging.

This checklist is process evidence, not a legal opinion. Final licensing and
trademark decisions remain human approval gates.

## Release attestation template

- [ ] Sources were derived from public standards, approved contracts, or
  independently authored tests.
- [ ] No implementation source, private artifact, or restricted test data was
  copied from an unrelated browser engine.
- [ ] Every dependency has a version, source, purpose, license, and integrity
  record in `security/supply-chain-manifest.json`.
- [ ] Generated outputs identify their canonical source and generator.
- [ ] SBOM, provenance, checksums, signatures, and license review are attached
  before a release artifact is promoted.
- [ ] Any suspected contamination or license concern is escalated through
  H-LEGAL-01 and is not silently waived.
