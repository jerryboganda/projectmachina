---
title: "Licensing, Clean-Room, and Intellectual Property"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define independent implementation controls, dependency licensing, contribution provenance, and release review."
---

# Licensing, Clean-Room, and Intellectual Property

## Baseline

Project Machina is an independent implementation. Lightpanda is useful public product/architecture research, but its browser repository is AGPL-3.0-only. Do not copy source, tests, comments, generated code, or distinctive internal implementation into a differently licensed core without authorized legal decision.

## Clean-room rules

- Implement from public standards, official protocol documentation, independently written requirements, observed interoperable behavior where legally appropriate, and original design.
- Record source/provenance of imported code, tests, fixtures, schemas, and data.
- Do not paste third-party source into coding-agent prompts or repository unless license and use are approved.
- Review suspiciously similar generated code before merge.
- Keep competitor benchmark fixtures/scripts license compliant.

## Project license decision

Recommended target: permissive core license plus separately licensed managed service/control-plane additions as approved. The exact license is a human/legal gate before public distribution.

## Dependencies

Maintain SBOM and license inventory. Define allowed, review-required, and prohibited license categories with counsel. Network-copyleft, strong-copyleft, noncommercial, source-available, field-of-use, and unknown licenses require explicit review.

## Standards/protocol material

Use official schemas/specifications according to their terms. Preserve notices. Generated code retains required attribution/licensing.

## Contributions

Use Developer Certificate of Origin or contributor agreement as selected. Contributors certify right to submit and identify third-party material. Security-sensitive or clean-room concerns receive restricted review.

## Branding

“Project Machina” is a working codename. Complete trademark, domain, package, and binary-name review before public release. Do not imply affiliation with Lightpanda, Chromium/Google, OpenAI, Anthropic, GitHub, or standards bodies.

## Release gate

Legal owner reviews project license, SBOM/licenses/notices, clean-room record, branding, redistribution of V8/Chromium and test corpora, and third-party service terms before beta/GA distribution.
