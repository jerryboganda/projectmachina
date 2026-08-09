# M2-T06 V8 Toolchain Provisioning — Feasibility Findings

> Produced by a wave-2 platform research agent. Read-only; nothing installed/downloaded.
> Use directly as a BLOCKERS.md-style entry or an owner briefing for the M2-T06 go/no-go call.

## Host facts (this Windows dev machine)

36 logical CPUs (Xeon E5 v4 class), ~32GB RAM, 438GB free on D:. `gn`/`gclient`/depot_tools: **absent**. `ninja` 1.13.2 and `clang`/`clang++` 21.1.6 present and version-matched to `toolchains/versions.toml` — but the clang is a Swift-toolchain bundle, not a purpose-built Chromium clang; V8's own build normally pulls its own pinned clang via `gclient runhooks` regardless, so this match may be coincidental. **Better news than assumed**: MSVC Build Tools 2022 (17.14.34) + Windows 10 SDK 10.0.26100.0 are already installed (found via `vswhere`) — the classic admin/UAC-gated blocker (BLK-002/BLK-003's Docker pattern) likely does NOT recur for the non-sanitized Windows build specifically. `cmake` 4.4.2 and `git` 2.53.0 also match pinned versions.

**VPS caveat**: the 134GB-free-disk figure cited in this session's earlier VPS work is orchestrator-reported from a different branch (`origin/chore/m1-vps-runtime-evidence`, not yet on main) — not independently re-verified by this agent. Confirm with a real `df -h`/`nproc`/`free -h` before budgeting VPS build time, don't carry the number forward blindly.

## 1. depot_tools install — not a doomed UAC-style install

Just `git clone .../depot_tools.git <dir>` + PATH prepend (user-scope env var, no installer/admin/UAC). First `gclient`/`gn` invocation self-bootstraps a private Python/git inside the depot_tools directory (plain user-writable file writes) — structurally different risk class from the Docker Desktop installs that failed at UAC. High confidence this step itself isn't blocked, though not live-tested here (out of scope for a read-only pass).

## 2. Disk/bandwidth

No live web access in this pass, so these are pattern-based estimates, not sourced figures — verify against v8.dev before budgeting: V8-only checkout (excludes `//chrome`/Blink) plausibly a few GB to ~6-8GB on disk for source+DEPS; network transfer plausibly 2-5GB; release `v8_monolith` build output several GB to ~15GB. Given 438GB free locally and a reported 134GB on the VPS, **disk is not the binding constraint on either host** — bandwidth and wall-clock time are.

## 3. Build time — honest estimate

**30-90 minutes wall-clock** for the release monolith build alone, once source is synced, on 36 cores/32GB RAM. **RAM is a real risk, not a formality**: 32GB with 36-way ninja parallelism is right at the edge for V8/Chromium-scale builds — should run with capped `-j` (~half of nproc), not ninja's full-core autodetect, or risk OOM/swapping turning a 45-minute build into a failure. Combining first-time depot_tools bootstrap + `gclient sync` (20-60+ min) + build + inevitable first-attempt GN-args troubleshooting = **realistically a multi-hour session with real risk of a second attempt**, not a 10-minute task, and not confidently a predictable "2 hour, walk away" task on the first try either. **Should not be attempted inside a single bounded agent turn with no checkpoint/resume plan.**

## 3b. Sanitizer build — separate platform leg, not just "2x cost"

Per the T06 design's §4, sanitizers are **Linux+Clang only** — MSVC ASan is immature and explicitly excluded. So the real fact isn't "does it double the Windows build time," it's **the sanitizer artifact cannot be built on this Windows host at all, full stop**. On Linux, ASan/UBSan instrumented builds are typically 1.3-2x slower to compile with larger binaries — fair "roughly doubles" characterization, but only *after* moving to a separate Linux/Clang leg.

## 4. Windows GN/V8 build support — better than assumed, still not trivial

MSVC/SDK prerequisite is already satisfied here (unlike the Docker case), so the admin-gate risk pattern likely doesn't recur for the non-sanitized build. V8's GN build supports Windows via `clang-cl` (Chromium's required Windows toolchain); `DEPOT_TOOLS_WIN_TOOLCHAIN=0` env var lets depot_tools auto-detect the local VS install (no install step). But: GN/depot_tools normally pulls its *own* pinned clang via `tools/clang/scripts/update.py` rather than trusting whatever's on PATH, so the found clang's version match may be coincidental; first-time Windows GN/ninja V8 builds routinely hit toolchain-detection friction even with VS present. **Net: real documented path, closer to ready than expected, but still first-attempt-uncertain — not "run one command and it works."**

## 5. Is the VPS a better venue?

**Yes, but only for the sanitizer leg, and only once its specs are actually confirmed.** The sanitizer build requires Linux+Clang — no Windows alternative exists, so for that half of T06 the VPS (or some Linux box) is mandatory, not merely preferable. For the non-sanitized build, the calculus is closer than expected since this Windows host already clears the MSVC/SDK prerequisite — plausible to attempt locally without hitting a new admin gate. The VPS's Docker/Ubuntu 24.04 environment is a clean, disposable, owner-controlled Linux box (per the earlier merged VPS evidence) — genuine advantage for the sanitizer build specifically (zero local-admin-gate risk, since it's already-trusted owner infrastructure). **Caveat**: confirm real disk/CPU/RAM on the VPS before budgeting time there rather than carrying forward an unverified number. Nothing was attempted on the VPS in this pass.

## 6. Recommendation — split M2-T06 into two separately-tracked pieces

**Do not attempt full M2-T06 (toolchain provisioning + C++ bridge/Rust facade code) as one bounded agent task.**

- **Step A — "Provision V8 build toolchain"** (infrastructure/prerequisite, human-owner-aware, multi-hour, two legs):
  - *Leg A1 (Windows, local, non-sanitized artifact)*: depot_tools clone → PATH → `DEPOT_TOOLS_WIN_TOOLCHAIN=0` → `fetch v8` pinned to `13.1.201.12` → `gclient sync` → `gn gen` + `ninja v8_monolith` release. Checkpointable (sync and build independently resumable); real risk of first-attempt friction despite VS Build Tools already present.
  - *Leg A2 (Linux/VPS, sanitized artifact)*: same checkout on the VPS or another Linux box, `is_asan=true is_ubsan=true` GN args. **Requires explicit owner awareness before touching VPS resources** for a multi-GB, potentially hours-long job on shared infrastructure already hosting ~20 other live containers — should be a logged, owner-visible action, not silently kicked off.
  - Both legs produce the pinned artifact + SHA/GN-args provenance the design's §1 calls for (`toolchains/V8_PROVENANCE.md`). Track as its own task ID, separate from M2-T06's acceptance criteria; success = "two build artifact sets exist with recorded provenance," reviewed and closed before the C++/Rust work begins.
- **Step B — "Write the C++ bridge + Rust facade"** (the actual M2-T06 code, per `.agent-state/design/M2-T06-v8-bridge-design.md` §2/§3/§5/§6): only starts once Step A's artifacts exist. Normal bounded implementation task once `MACHINA_V8_ROOT` points at real built artifacts — the design is already thorough and ready to execute against.

This keeps the multi-hour, multi-GB, environment-uncertain toolchain acquisition from silently consuming a code-review/authorship turn budget, and gives the owner an explicit point to approve VPS build-hours before they're spent.

## Files referenced

`.agent-state/design/M2-T06-v8-bridge-design.md` · `agents/BLOCKERS.md` (BLK-002, BLK-003) · `toolchains/versions.toml` · `.agent-state/evidence/M1-T12.md`/`.claim.md`/`.claim.json` · `origin/chore/m1-vps-runtime-evidence` branch → `.agent-state/evidence/M1-T12-vps-runtime.md` (not yet on main, read via `git show`).
