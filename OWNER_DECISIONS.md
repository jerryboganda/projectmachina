---
title: "Owner Decision Popups"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Capture owner-controlled product and delivery decisions with safe recommended defaults."
---

# Owner Decision Popups

Markdown cannot open operating-system dialog boxes, so this file uses collapsible `<details>` panels as portable popup-style decision cards. An automation agent may proceed with every option marked **Recommended default** unless the owner edits `Selected:`.

<details open>
<summary><strong>Decision 01 — Repository and license strategy</strong></summary>

**Selected:** `D01-B`

- `D01-A` — Fork Lightpanda under its applicable license obligations.
- `D01-B` — **Recommended default:** independent clean-room implementation, permissive-license target, Chromium only as an external compatibility runtime.
- `D01-C` — Fully proprietary core.

**Why:** maximizes architectural and commercial freedom while reducing source-contamination and network-copyleft risk. Final licensing still requires counsel review.
</details>

<details>
<summary><strong>Decision 02 — Primary launch workload</strong></summary>

**Selected:** `D02-C`

- `D02-A` — AI browser agents only.
- `D02-B` — Crawling and structured extraction only.
- `D02-C` — **Recommended default:** AI agents and extraction first; testing compatibility second.
- `D02-D` — Broad browser replacement immediately.
</details>

<details>
<summary><strong>Decision 03 — Core implementation</strong></summary>

**Selected:** `D03-C`

- `D03-A` — Zig throughout.
- `D03-B` — C++ throughout.
- `D03-C` — **Recommended default:** Rust core plus a narrow, audited C++ bridge to V8.
- `D03-D` — Rust with another JavaScript engine selected after prototype comparison.
</details>

<details>
<summary><strong>Decision 04 — Frontend</strong></summary>

**Selected:** `D04-A`

- `D04-A` — **Recommended default:** Svelte 5 + SvelteKit + TypeScript; static prerender for public/docs surfaces and server deployment for the authenticated console.
- `D04-B` — Plain Svelte + Vite for an embedded-only console.
- `D04-C` — SolidStart.
- `D04-D` — No graphical console before beta.

**Why:** Svelte compiles component logic, SvelteKit supplies routing, loading, server rendering, adapters, and deployment conventions, while retaining a small client footprint.
</details>

<details>
<summary><strong>Decision 05 — Agent concurrency</strong></summary>

**Selected:** `D05-B`

- `D05-A` — One implementation agent.
- `D05-B` — **Recommended default:** two implementation agents in separate worktrees; one independent reviewer per merge.
- `D05-C` — Four or more implementation agents from day one.

**Why:** two agents capture useful parallelism while keeping contract churn and merge conflicts manageable. Increase only after ownership telemetry proves low collision rates.
</details>

<details>
<summary><strong>Decision 06 — Testing cadence</strong></summary>

**Selected:** `D06-B`

- `D06-A` — No tests until the project is feature complete.
- `D06-B` — **Recommended default:** minimal risk-based fast gates per task, scheduled medium suites, exhaustive heavy certification once at M9.
- `D06-C` — Full test suite on every change.

**Why:** end-only testing tends to accumulate incompatible assumptions and makes failures expensive to localize. The recommended mode keeps inner-loop checks small while consolidating costly suites.
</details>

<details>
<summary><strong>Decision 07 — Isolation tiers</strong></summary>

**Selected:** `D07-C`

- `D07-A` — Shared process only.
- `D07-B` — One process per session only.
- `D07-C` — **Recommended default:** shared-performance, dedicated-process, and hardened container/microVM tiers.
</details>

<details>
<summary><strong>Decision 08 — Managed service scope</strong></summary>

**Selected:** `D08-C`

- `D08-A` — Local binary only.
- `D08-B` — Managed cloud only.
- `D08-C` — **Recommended default:** local binary, self-hosted deployment, and managed cloud using one protocol surface.
</details>

<details>
<summary><strong>Decision 09 — Data residency and regions</strong></summary>

**Selected:** `D09-A`

- `D09-A` — **Recommended default:** one launch region, region-aware data model, no cross-region session migration until controls are certified.
- `D09-B` — Multi-region active-active at beta.
- `D09-C` — Self-hosted only until GA.

**Human approval required before production:** choose launch region, retention periods, subprocessors, and contractual residency commitments.
</details>

<details>
<summary><strong>Decision 10 — Public benchmark claims</strong></summary>

**Selected:** `D10-A`

- `D10-A` — **Recommended default:** publish only reproducible equal-fidelity, equal-success comparisons reviewed by an independent owner.
- `D10-B` — Publish internal best-case multipliers.

No agent may approve its own public performance claim.
</details>

## Human-required decisions

The autonomous loop must pause only the affected workstream for:

- final company/product name and trademark clearance;
- license policy and legal approval;
- production cloud accounts and spending ceiling;
- production secrets and privileged access;
- data retention and geographic commitments;
- acceptance of material security risk;
- public benchmark or compatibility claims;
- beta, release-candidate, and general-availability launch authorization.
