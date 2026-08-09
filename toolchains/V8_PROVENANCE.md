# V8 Build Provenance

> Referenced by `.agent-state/design/M2-T06-v8-bridge-design.md` §1 ("resolve to an
> immutable commit SHA; record SHA + DEPS-lock digest + source-tree SHA-256 + exact
> GN args in `toolchains/versions.toml` or a sibling `toolchains/V8_PROVENANCE.md`").
> This file is that sibling record.

## Status

**Both artifact sets accepted.** See "Accepted builds" below. Real binaries were
downloaded from the GitHub Actions run and their SHA-256 independently
recomputed (not just copied from `provenance.json`) before acceptance; both
matched exactly. `MACHINA_V8_ROOT` for M2-T06 proper may now point at these
artifacts once re-downloaded from the same run (or a re-run at the same pinned
commit — see `resolved_commit` below).

## Scope

Two artifact sets are required per `toolchains/versions.toml`'s pinned
`[v8].version` before M2-T06 proper (the C++ bridge / Rust facade implementation
described in `.agent-state/design/M2-T06-v8-bridge-design.md`) may begin:

1. **Non-sanitized** — release, optimized `v8_monolith` static library, used for
   normal functional builds of `cpp/v8-bridge/`.
2. **Sanitized (ASan+UBSan)** — `v8_monolith` built with `is_asan=true
   is_ubsan=true`, used by the `bridge-sanitizer` CI job design §4 requires.
   This artifact set can only be built on Linux+Clang (MSVC ASan is immature and
   explicitly excluded); there is no Windows equivalent.

Both are produced by `.github/workflows/v8-toolchain-build.yml`
(`workflow_dispatch`-only — an explicit, owner-triggered action, not run on every
push/PR, because a full V8 checkout + build is multi-GB and can take from well
under an hour to several hours depending on runner class).

## How this file gets populated

The workflow does **not** commit directly to this file. Each run uploads a
`provenance.json` (per artifact set) as a workflow artifact alongside the built
library, containing every field below. When a build is reviewed and accepted as
the artifact `MACHINA_V8_ROOT` should point at for M2-T06 proper, a human or a
follow-up task copies that `provenance.json`'s fields into the tables below as a
dated addendum. This keeps the repository's durable provenance record reviewed
and intentional rather than auto-written by an unattended CI job.

Workflow artifact names (from `actions/upload-artifact`):

- `v8-monolith-non-sanitized-windows-x64` → contains `v8_monolith.lib` +
  `provenance.json`
- `v8-monolith-asan-ubsan-linux-x64` → contains `libv8_monolith.a` +
  `provenance.json`

## Provenance schema

Each artifact set's record must include:

| Field | Description |
|---|---|
| `artifact_set` | `non-sanitized` or `sanitized-asan-ubsan` |
| `v8_version_tag` | The V8 tag requested (should match `toolchains/versions.toml` `[v8].version`) |
| `resolved_commit` | Immutable commit SHA the tag resolved to at build time |
| `source_repo` | `https://chromium.googlesource.com/v8/v8.git` |
| `gn_args` | Exact GN args string used for `gn gen` |
| `build_target` | `v8_monolith` |
| `artifact_file` | `v8_monolith.lib` (Windows) or `libv8_monolith.a` (Linux) |
| `artifact_sha256` | SHA-256 of the built artifact file |
| `build_timestamp` | UTC ISO-8601 build completion time |
| `runner_os` | Runner OS/version |
| `runner_cpu` | Runner CPU model |
| `runner_cpu_cores` | Runner logical core count |
| `runner_mem_gb` | Runner total RAM in GB |
| `github_run_id` / `github_run_url` | The Actions run that produced the artifact |
| `workflow_file` | `.github/workflows/v8-toolchain-build.yml` |

## Accepted builds

Accepted 2026-08-09 from `.github/workflows/v8-toolchain-build.yml` run
[31307851332](https://github.com/jerryboganda/projectmachina/actions/runs/31307851332)
(branch `agent/v8-toolchain-fix-round2`, merged via PR #35). Both artifacts
were downloaded and their SHA-256 independently recomputed against the actual
binary before acceptance — not transcribed from `provenance.json` alone.

### Non-sanitized

```
artifact_set:       non-sanitized
v8_version_tag:      13.1.201.12
resolved_commit:      ad8b9f1f6027eb4c29437c04a1d2ce2b82467ba5
source_repo:           https://chromium.googlesource.com/v8/v8.git
gn_args:                is_debug=false target_cpu="x64" is_clang=true v8_monolithic=true v8_use_external_startup_data=false is_component_build=false v8_enable_i18n_support=false symbol_level=0
build_target:             v8_monolith
artifact_file:             v8_monolith.lib
artifact_sha256:            de25b37e08d24efedd699c7f7fd5e6a3a99c3c105cd166c3367fbbc6bf666039
build_timestamp:              2026-08-09T11:41:32Z
runner_os:                     Microsoft Windows Server 2025 Datacenter 10.0.26100
runner_cpu:                     unknown — provenance.json field bug, see "Known issues" below
runner_cpu_cores:                 unknown — same bug
runner_mem_gb:                     unknown — same bug
github_run_id / url:                 31307851332 / https://github.com/jerryboganda/projectmachina/actions/runs/31307851332
workflow_file:                         .github/workflows/v8-toolchain-build.yml
```

### Sanitized (ASan+UBSan)

```
artifact_set:       sanitized-asan-ubsan
v8_version_tag:      13.1.201.12
resolved_commit:      ad8b9f1f6027eb4c29437c04a1d2ce2b82467ba5
source_repo:           https://chromium.googlesource.com/v8/v8.git
gn_args:                is_debug=false target_cpu="x64" is_clang=true v8_monolithic=true v8_use_external_startup_data=false is_component_build=false v8_enable_i18n_support=false is_asan=true is_ubsan=true symbol_level=1
build_target:             v8_monolith
artifact_file:             libv8_monolith.a
artifact_sha256:            0652ca3a9c74717bd491b031c8815f5414ef8df8debcbb1fb4ca6f915d810d64
build_timestamp:              2026-08-09T11:16:54Z
runner_os:                     Ubuntu 24.04.4 LTS
runner_cpu:                     AMD EPYC 9V74 80-Core Processor
runner_cpu_cores:                 4
runner_mem_gb:                     15.6
github_run_id / url:                 31307851332 / https://github.com/jerryboganda/projectmachina/actions/runs/31307851332
workflow_file:                         .github/workflows/v8-toolchain-build.yml
```

## Known issues

- The non-sanitized (Windows) leg's `provenance.json` has `null` for
  `runner_cpu`/`runner_cpu_cores`/`runner_mem_gb` — a `$CPU_DESC` vs.
  `$env:CPU_DESC` variable-reference bug in the workflow's Package step,
  unrelated to build correctness (the artifact and its checksum are real and
  verified; only this metadata is affected). Worth a small follow-up fix to
  the workflow. Not tracked as a blocker since it doesn't affect artifact
  usability.
- Artifact retention: GitHub Actions artifacts from this run expire
  2026-08-23. Before then, either re-run the workflow at the same pinned
  commit to regenerate them, or copy the binaries somewhere durable if
  `MACHINA_V8_ROOT` needs to reference them past that date.
