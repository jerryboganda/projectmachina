# Evidence: M2-T06 Step A — V8 toolchain build via GitHub Actions

Task: author `.github/workflows/v8-toolchain-build.yml` (`workflow_dispatch`-only)
per `.agent-state/design/M2-T06-toolchain-feasibility.md`'s Step A recommendation.
Branch: `agent/v8-toolchain-github-actions`. PR:
https://github.com/jerryboganda/projectmachina/pull/33

## What was delivered

- `.github/workflows/v8-toolchain-build.yml` — `on: workflow_dispatch` only (no
  push/PR/schedule trigger). Two independent jobs:
  - `build-non-sanitized` (`windows-latest`) — release `v8_monolith` via
    depot_tools + `fetch v8` + `gclient sync` + `gn gen` + `ninja`.
  - `build-sanitized` (`ubuntu-latest`, mandatory) — same fetch/sync path, GN
    args `is_asan=true is_ubsan=true`.
  Both: resolve the pinned V8 tag to an immutable commit SHA, check/reclaim
  runner disk space before the multi-GB sync+build, use per-step timeouts,
  checksum the artifact, write a `provenance.json`, upload via
  `actions/upload-artifact`.
- `toolchains/V8_PROVENANCE.md` — schema/template for the provenance record
  the design doc §1 requires; documents that the workflow's per-run
  `provenance.json` artifacts are the source of truth, copied into this file
  only once a build is reviewed/accepted.

## Validation performed

### 1. actionlint (static workflow validation)

No `actionlint`/`yamllint` was already vendored in this repo's tooling (checked
`package.json` scripts and `.github/workflows/fast-gate.yml`'s own gate — it
doesn't lint its own YAML with a dedicated tool either). Downloaded the
official `actionlint` v1.7.12 Windows release binary and ran it directly:

```
$ actionlint.exe .github/workflows/v8-toolchain-build.yml
(no output — exit 0)

$ actionlint.exe .github/workflows/fast-gate.yml   # sanity check against a known-good file
(no output — exit 0)
```

Both pass clean. `shellcheck` was not available in this environment, so
actionlint's embedded shellcheck pass over the `run:` blocks did not execute;
the embedded bash/PowerShell blocks were otherwise reviewed by hand and the
TOML-parsing logic (`awk`/PowerShell loop over `toolchains/versions.toml`) was
independently exercised against the real file:

```
$ awk '/^\[v8\]/{f=1;next} /^\[/{f=0} f && /^version *=/{gsub(/[" ]/,"",$3); print $3}' toolchains/versions.toml
13.1.201.12
```

### 2. V8 tag → commit SHA resolution (the same logic the workflow runs)

```
$ git ls-remote https://chromium.googlesource.com/v8/v8.git refs/tags/13.1.201.12
ad8b9f1f6027eb4c29437c04a1d2ce2b82467ba5	refs/tags/13.1.201.12

$ git ls-remote https://chromium.googlesource.com/v8/v8.git 'refs/tags/13.1.201.12*'
ad8b9f1f6027eb4c29437c04a1d2ce2b82467ba5	refs/tags/13.1.201.12
ad8b9f1f6027eb4c29437c04a1d2ce2b82467ba5	refs/tags/13.1.201.12-pgo
```

No `^{}` peeled line appears, confirming `13.1.201.12` is a lightweight tag
pointing directly at commit `ad8b9f1f6027eb4c29437c04a1d2ce2b82467ba5` — the
workflow's peel-detection logic correctly falls back to using the tag SHA
directly in this case. This resolution step works as designed.

### 3. fast-gate on the PR

```
$ gh pr checks 33 --repo jerryboganda/projectmachina
fast-gate	pass	54s	https://github.com/jerryboganda/projectmachina/actions/runs/31303521373/job/93220045178
```

Passes — the two new files (a workflow under `.github/workflows/` and a new
doc under `toolchains/`) do not affect any existing repository-policy check.

## Attempting a live trigger — blocked by a pre-existing, unrelated repo config issue

Per the task, I attempted to actually trigger the workflow:

```
$ gh workflow list --repo jerryboganda/projectmachina
fast-gate	active	330178351
```

`v8-toolchain-build.yml` does not appear — GitHub only lists/dispatches
`workflow_dispatch` workflows that exist on the repository's **configured
default branch**. I confirmed this is the actual blocker, not just "PR not
yet merged":

```
$ gh workflow run v8-toolchain-build.yml --ref agent/v8-toolchain-github-actions
HTTP 404: workflow v8-toolchain-build.yml not found on the default branch
(https://api.github.com/repos/jerryboganda/projectmachina/actions/workflows/v8-toolchain-build.yml)

$ gh api -X POST repos/jerryboganda/projectmachina/actions/workflows/v8-toolchain-build.yml/dispatches -f ref=agent/v8-toolchain-github-actions
{"message":"Not Found", ...}  (HTTP 404, raw REST API, same result)
```

The workflow file **is** present and pushed on
`agent/v8-toolchain-github-actions` (verified: `git show
origin/agent/v8-toolchain-github-actions:.github/workflows/v8-toolchain-build.yml`
returns the file) — the 404 is not "branch doesn't have the file", it is
GitHub's documented `workflow_dispatch` requirement that the workflow
definition exist on the repository's default branch specifically, regardless
of which `ref` is passed to run against.

I then checked what this repository's actual configured default branch is:

```
$ gh api repos/jerryboganda/projectmachina --jq '.default_branch'
agent/M0-T01-bootstrap
```

**This is a real, pre-existing repository misconfiguration, unrelated to this
task**: the GitHub repo's `default_branch` setting still points at
`agent/M0-T01-bootstrap` (presumably left over from the very first bootstrap
task/branch that was pushed when the repo was created), even though every
subsequent PR in this repo's history (`#22`–`#30`, confirmed via `gh pr list
--state merged`) targets `main` as the working trunk. `main` itself is not
GitHub-branch-protected:

```
$ gh api repos/jerryboganda/projectmachina/branches/main/protection
{"message":"Branch not protected", "status":"404"}
```

Because of this, `workflow_dispatch` cannot be used for **any new** workflow
in this repository — including this one — until either (a) the repo's
`default_branch` setting is corrected to `main`, or (b) the workflow file is
merged onto whatever branch is actually configured as default.

### Why I did not work around this myself

- Merging PR #33 into `main` would not by itself fix the problem (the
  configured default branch is `agent/M0-T01-bootstrap`, not `main`), so it
  would not have produced a live run either.
- Changing the repository's `default_branch` setting is a repo-wide
  administrative action well outside this task's owned scope
  (`.github/workflows/v8-toolchain-build.yml`, `toolchains/V8_PROVENANCE.md`),
  affects every future PR/CI trigger resolution in the repository, and is
  exactly the class of protected/administrative-settings change the operating
  instructions require a documented human gate for. It is not mine to change
  unilaterally based on a task-agent's request.
- Checking `merged_by` on the repo's actual merge history confirms every prior
  PR in this repository was merged by a human account (`jerryboganda` or
  `manwara575-star`), never by the automation identity, across the full
  history sampled (`#1`, `#22`, `#23`, `#28`, `#30`):

  ```
  $ gh api repos/jerryboganda/projectmachina/pulls/30 --jq '.merged_by.login'
  jerryboganda
  $ gh api repos/jerryboganda/projectmachina/pulls/28 --jq '.merged_by.login'
  jerryboganda
  $ gh api repos/jerryboganda/projectmachina/pulls/23 --jq '.merged_by.login'
  manwara575-star
  $ gh api repos/jerryboganda/projectmachina/pulls/22 --jq '.merged_by.login'
  manwara575-star
  $ gh api repos/jerryboganda/projectmachina/pulls/1 --jq '.merged_by.login'
  jerryboganda
  ```

  Consistent with this established pattern, PR #33 was left open for the
  owner to merge rather than self-merged.
- I did not push this workflow to the stale `agent/M0-T01-bootstrap` branch
  either, as that branch is outside this task's owned scope and is not the
  branch anyone actually develops against.

**Net result: I was not able to obtain a live GitHub Actions run of this
workflow within this task.** This is reported honestly rather than worked
around. Recommended follow-up for the owner: fix the repo's `default_branch`
setting to `main` (unrelated pre-existing issue, worth its own tracked fix
regardless of this task), then merge PR #33, then trigger
`v8-toolchain-build.yml` for real validation.

## Honest assessment

- **Static validation**: the workflow is syntactically valid (actionlint
  clean), the version-resolution logic was independently verified against the
  real pinned tag and the real `toolchains/versions.toml` content, and the
  fetch/build recipe (`fetch v8` → checkout pinned SHA → `gclient sync` → `gn
  gen` with the documented v8.dev "embedding V8" minimal arg set → `ninja -C
  ... v8_monolith`) matches V8's own documented external-contributor build
  flow, not an invented one.
- **Not empirically confirmed**: whether `gclient sync`/depot_tools bootstrap
  actually succeeds on a GitHub-hosted runner, and whether the build completes
  within the 6-hour job timeout, remain unverified — no live run occurred, for
  the repository-configuration reason above, not because of a flaw found in
  the workflow itself.
- **Real risk this workflow's design tries to account for, but that could not
  be empirically checked**: `.agent-state/design/M2-T06-toolchain-feasibility.md`'s
  30–90 minute build-time estimate was benchmarked on a 36-logical-core /
  32GB-RAM local machine. Standard GitHub-hosted `windows-latest`/`ubuntu-latest`
  runners are far smaller (typically 4 logical cores / 16GB RAM). It is
  realistic — not merely possible — that the actual build time on a
  GitHub-hosted runner will be substantially longer than that estimate, and
  combined with `gclient sync` time (20–60+ minutes per the feasibility doc,
  also likely longer on a smaller/shared runner's bandwidth and disk), there
  is a real chance the non-sanitized Windows leg in particular (first-attempt
  GN/toolchain-detection friction, per the feasibility doc) could approach or
  exceed the 6-hour job ceiling on a first attempt. The workflow's per-step
  timeouts are sized to fail clearly and early rather than silently hang to
  that ceiling, but they do not solve a genuinely-too-slow build — that would
  require the checkpoint/resume architecture the feasibility doc explicitly
  recommended *not* attempting until a real run's actual numbers justify it.
- **Verdict**: the approach (depot_tools + `fetch`/`gclient`/`gn`/`ninja`,
  entirely on GitHub-hosted runners, manually triggered) is the correct,
  standard, unexotic one and matches the design/feasibility docs' own
  recommendation. Whether it actually produces usable artifacts within budget
  is genuinely unknown until a live run happens — which requires the owner to
  either fix the `default_branch` setting or merge PR #33 (or both), then
  dispatch it. I could not do that within this task's scope.
