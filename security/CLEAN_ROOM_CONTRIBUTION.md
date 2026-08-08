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
