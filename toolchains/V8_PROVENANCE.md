# V8 Build Provenance

> Referenced by `.agent-state/design/M2-T06-v8-bridge-design.md` §1 ("resolve to an
> immutable commit SHA; record SHA + DEPS-lock digest + source-tree SHA-256 + exact
> GN args in `toolchains/versions.toml` or a sibling `toolchains/V8_PROVENANCE.md`").
> This file is that sibling record.

## Status

**Template / placeholder.** No V8 build has been accepted into this record yet.
This file documents the schema and the process; it does not itself assert that a
build has happened. See "How this file gets populated" below.

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

_None yet. Populate this section once a workflow run's artifacts have been
reviewed and accepted for use by M2-T06 proper._

### Non-sanitized

```
(placeholder — no accepted build)
```

### Sanitized (ASan+UBSan)

```
(placeholder — no accepted build)
```
