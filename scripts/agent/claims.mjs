import { constants } from "node:fs";
import { realpathSync } from "node:fs";
import {
  access,
  mkdir,
  readdir,
  readFile,
  rename,
  rm,
  stat,
  writeFile
} from "node:fs/promises";
import { randomUUID } from "node:crypto";
import { join, relative, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

export const CLAIM_SCHEMA_VERSION = 1;
export const DEFAULT_LEASE_MINUTES = 90;
export const DEFAULT_GRACE_MINUTES = 20;
export const STALE_LOCK_MINUTES = 30;

function rootFromModule() {
  return fileURLToPath(new URL("../..", import.meta.url));
}

function claimStoreRoot(root) {
  const result = spawnSync("git", ["rev-parse", "--git-common-dir"], {
    cwd: root,
    encoding: "utf8",
    shell: false,
    stdio: "pipe"
  });
  if (!result.error && result.status === 0 && result.stdout.trim()) {
    return join(resolve(root, result.stdout.trim()), "machina-claims");
  }
  return join(root, ".agent-state");
}

function claimsDirectory(root) {
  return join(claimStoreRoot(root), "claims");
}

function lockDirectory(root) {
  return join(claimStoreRoot(root), "claims.lock");
}

function claimPath(root, taskId) {
  const safeTaskId = taskId.replace(/[^A-Za-z0-9._-]/g, "_");
  return join(claimsDirectory(root), `${safeTaskId}.json`);
}

function canonicalPath(root, path) {
  const absolute = resolve(root, path);
  let real = absolute;
  try {
    real = realpathSync.native(absolute);
  } catch {
    real = absolute;
  }
  return process.platform === "win32" ? real.toLowerCase() : real;
}

function validateWorktree(root, branch, worktree, allowNonGit) {
  const repository = spawnSync("git", ["rev-parse", "--show-toplevel"], {
    cwd: root,
    encoding: "utf8",
    shell: false,
    stdio: "pipe"
  });
  if (repository.error || repository.status !== 0) {
    if (allowNonGit) {
      return;
    }
    throw new Error("claim worktree validation requires a Git repository");
  }
  const result = spawnSync("git", ["worktree", "list", "--porcelain"], {
    cwd: root,
    encoding: "utf8",
    shell: false,
    stdio: "pipe"
  });
  if (result.error || result.status !== 0) {
    throw new Error("unable to verify Git worktree ownership");
  }
  const expectedPath = canonicalPath(root, worktree);
  const blocks = result.stdout.trim().split(/\r?\n\r?\n/).filter(Boolean);
  const match = blocks.find((block) => {
    const lines = block.split(/\r?\n/);
    const path = lines.find((line) => line.startsWith("worktree "))?.slice(9);
    const actualBranch = lines.find((line) => line.startsWith("branch "))?.slice(7);
    return (
      path &&
      actualBranch === `refs/heads/${branch}` &&
      canonicalPath(root, path) === expectedPath
    );
  });
  if (!match) {
    throw new Error(`worktree is not registered on the claimed branch: ${worktree}`);
  }
}

export function normalizeScope(scope, root) {
  if (typeof scope !== "string" || scope.trim().length === 0) {
    throw new Error("write scope must be a non-empty path glob");
  }

  const normalized = scope.trim().replaceAll("\\", "/");
  if (normalized.startsWith("/") || /^[A-Za-z]:/.test(normalized)) {
    throw new Error(`write scope must be repository-relative: ${scope}`);
  }

  const segments = normalized
    .split("/")
    .filter((segment) => segment.length > 0 && segment !== ".");
  if (segments.some((segment) => segment === "..")) {
    throw new Error(`write scope escapes repository root: ${scope}`);
  }

  const candidate = segments.join("/");
  const absolute = resolve(root, candidate);
  const relativePath = relative(resolve(root), absolute).replaceAll("\\", "/");
  if (relativePath.startsWith("../") || relativePath === "..") {
    throw new Error(`write scope escapes repository root: ${scope}`);
  }

  return candidate;
}

function literalPrefix(scope) {
  const wildcardIndex = scope.search(/[*?[\]]/);
  const prefix = wildcardIndex === -1 ? scope : scope.slice(0, wildcardIndex);
  return prefix.endsWith("/") ? prefix.slice(0, -1) : prefix;
}

export function scopesOverlap(left, right) {
  const foldCase = process.platform === "win32";
  const leftScope = (foldCase ? left.toLowerCase() : left).replaceAll("\\", "/");
  const rightScope = (foldCase ? right.toLowerCase() : right).replaceAll("\\", "/");
  const leftPrefix = literalPrefix(leftScope);
  const rightPrefix = literalPrefix(rightScope);
  const leftHasWildcard = /[*?[\]]/.test(leftScope);
  const rightHasWildcard = /[*?[\]]/.test(rightScope);

  if (leftPrefix.length === 0 || rightPrefix.length === 0) {
    return true;
  }

  return (
    leftPrefix === rightPrefix ||
    (leftHasWildcard && rightPrefix.startsWith(leftPrefix)) ||
    (rightHasWildcard && leftPrefix.startsWith(rightPrefix)) ||
    leftPrefix.startsWith(`${rightPrefix}/`) ||
    rightPrefix.startsWith(`${leftPrefix}/`)
  );
}

async function acquireLock(root) {
  await mkdir(claimStoreRoot(root), { recursive: true });
  const lockId = randomUUID();
  try {
    await mkdir(lockDirectory(root));
    await writeFile(
      join(lockDirectory(root), "owner.json"),
      `${JSON.stringify({
        lock_id: lockId,
        pid: process.pid,
        acquired_at: new Date().toISOString()
      })}\n`,
      "utf8"
    );
  } catch (error) {
    if (error?.code === "EEXIST") {
      let age = 0;
      try {
        age = Date.now() - (await stat(lockDirectory(root))).mtimeMs;
      } catch {
        age = 0;
      }
      if (age >= STALE_LOCK_MINUTES * 60 * 1000) {
        throw new Error("claim state has a stale lock; inspect and run recover-lock");
      }
      throw new Error("claim state is locked by another operation");
    }
    throw error;
  }
  return lockId;
}

async function releaseLock(root, lockId) {
  const owner = JSON.parse(await readFile(join(lockDirectory(root), "owner.json"), "utf8"));
  if (owner.lock_id !== lockId) {
    throw new Error("claim lock ownership changed before release");
  }
  const releasePath = `${lockDirectory(root)}.release-${lockId}`;
  await rename(lockDirectory(root), releasePath);
  await rm(releasePath, { recursive: true, force: true });
}

async function withLock(root, operation) {
  const lockId = await acquireLock(root);
  try {
    return await operation();
  } finally {
    await releaseLock(root, lockId);
  }
}

async function writeJsonAtomic(path, value, flag = "w") {
  const temporaryPath = `${path}.${randomUUID()}.tmp`;
  await writeFile(temporaryPath, `${JSON.stringify(value, null, 2)}\n`, {
    encoding: "utf8",
    flag
  });
  await rename(temporaryPath, path);
}

async function readClaimFiles(root) {
  const directory = claimsDirectory(root);
  try {
    await access(directory, constants.F_OK);
  } catch (error) {
    if (error?.code === "ENOENT") {
      return [];
    }
    throw error;
  }

  const entries = await readdir(directory, { withFileTypes: true });
  const claims = [];
  for (const entry of entries) {
    if (!entry.isFile() || !entry.name.endsWith(".json")) {
      continue;
    }
    claims.push(JSON.parse(await readFile(join(directory, entry.name), "utf8")));
  }
  return claims;
}

async function validateKnownTask(root, task) {
  const registryPath = join(root, ".agent-state", "task-registry.json");
  try {
    const registry = JSON.parse(await readFile(registryPath, "utf8"));
    if (!registry.tasks.some((entry) => entry.id === task)) {
      throw new Error(`unknown task ID: ${task}`);
    }
  } catch (error) {
    if (error?.code === "ENOENT") {
      return;
    }
    throw error;
  }
}

function isActiveClaim(claim) {
  return claim.status === "active";
}

function isExpiredClaim(claim, now = Date.now()) {
  return Date.parse(claim.lease_expires_at) <= now;
}

function isPastGracePeriod(claim, now = Date.now()) {
  return Date.parse(claim.grace_expires_at) <= now;
}

function validateIdentity(flags) {
  for (const name of ["task", "agent", "branch", "worktree"]) {
    if (!flags[name]) {
      throw new Error(`--${name} is required`);
    }
  }
}

export async function claimTask({
  root,
  task,
  agent,
  branch,
  worktree,
  writeScope,
  dependencies = [],
  leaseMinutes = DEFAULT_LEASE_MINUTES,
  graceMinutes = DEFAULT_GRACE_MINUTES,
  allowNonGit = false,
  now = new Date()
}) {
  validateIdentity({ task, agent, branch, worktree });
  await validateKnownTask(root, task);
  if (branch === "main" || branch === "master") {
    throw new Error("active task claims require a task branch, not the protected default branch");
  }
  if (canonicalPath(root, ".") === canonicalPath(root, worktree)) {
    throw new Error("active task claims require a separate worktree");
  }
  validateWorktree(root, branch, worktree, allowNonGit);
  if (!Array.isArray(writeScope) || writeScope.length === 0) {
    throw new Error("at least one write scope is required");
  }
  if (!Number.isFinite(leaseMinutes) || leaseMinutes <= 0) {
    throw new Error("lease duration must be a positive number of minutes");
  }
  if (!Number.isFinite(graceMinutes) || graceMinutes < 0) {
    throw new Error("grace duration must be a non-negative number of minutes");
  }

  const normalizedScopes = writeScope.map((scope) => normalizeScope(scope, root));
  return withLock(root, async () => {
    const claims = await readClaimFiles(root);
    const activeClaims = claims.filter(isActiveClaim);
    for (const existing of activeClaims) {
      if (existing.task_id === task) {
        throw new Error(`task already has an active claim: ${task}`);
      }
      for (const requestedScope of normalizedScopes) {
        if (existing.write_scope.some((scope) => scopesOverlap(scope, requestedScope))) {
          throw new Error(
            `write scope overlaps active claim ${existing.task_id}: ${requestedScope}`
          );
        }
      }
    }

    const claimedAt = now.toISOString();
    const claim = {
      schema_version: CLAIM_SCHEMA_VERSION,
      task_id: task,
      agent_id: agent,
      branch,
      worktree,
      write_scope: normalizedScopes,
      dependencies,
      claimed_at: claimedAt,
      heartbeat_at: claimedAt,
      lease_expires_at: new Date(
        now.getTime() + leaseMinutes * 60 * 1000
      ).toISOString(),
      grace_expires_at: new Date(
        now.getTime() + (leaseMinutes + graceMinutes) * 60 * 1000
      ).toISOString(),
      status: "active"
    };
    await mkdir(claimsDirectory(root), { recursive: true });
    const path = claimPath(root, task);
    try {
      const previous = JSON.parse(await readFile(path, "utf8"));
      if (previous.status === "active") {
        throw new Error(`task already has an active claim: ${task}`);
      }
      await rename(path, `${path}.${randomUUID()}.archive.json`);
    } catch (error) {
      if (error?.code !== "ENOENT") {
        throw error;
      }
    }
    await writeJsonAtomic(path, claim, "wx");
    return claim;
  });
}

export async function heartbeatTask({
  root,
  task,
  agent,
  leaseMinutes = DEFAULT_LEASE_MINUTES,
  now = new Date()
}) {
  if (!task || !agent) {
    throw new Error("--task and --agent are required");
  }

  return withLock(root, async () => {
    const path = claimPath(root, task);
    const claim = JSON.parse(await readFile(path, "utf8"));
    if (claim.status !== "active") {
      throw new Error(`claim ${task} is not active`);
    }
    if (claim.agent_id !== agent) {
      throw new Error(`claim ${task} belongs to another agent`);
    }
    if (isExpiredClaim(claim, now.getTime())) {
      throw new Error(`claim ${task} is expired; recover it after the grace period`);
    }
    const heartbeatAt = now.toISOString();
    const updated = {
      ...claim,
      heartbeat_at: heartbeatAt,
      lease_expires_at: new Date(
        now.getTime() + leaseMinutes * 60 * 1000
      ).toISOString()
    };
    updated.grace_expires_at = new Date(
      now.getTime() + (leaseMinutes + DEFAULT_GRACE_MINUTES) * 60 * 1000
    ).toISOString();
    await writeJsonAtomic(path, updated);
    return updated;
  });
}

export async function releaseTask({ root, task, agent, reason = "completed", now = new Date() }) {
  if (!task || !agent) {
    throw new Error("--task and --agent are required");
  }

  return withLock(root, async () => {
    const path = claimPath(root, task);
    const claim = JSON.parse(await readFile(path, "utf8"));
    if (claim.status !== "active") {
      throw new Error(`claim ${task} is not active`);
    }
    if (claim.agent_id !== agent) {
      throw new Error(`claim ${task} belongs to another agent`);
    }
    if (isExpiredClaim(claim, now.getTime())) {
      throw new Error(`claim ${task} is expired; recover it after the grace period`);
    }
    const released = {
      ...claim,
      status: "released",
      release_reason: reason,
      released_at: now.toISOString()
    };
    await writeJsonAtomic(path, released);
    return released;
  });
}

export async function recoverExpiredTask({
  root,
  task,
  actor,
  reason,
  now = new Date()
}) {
  if (!task || !actor || !reason) {
    throw new Error("--task, --actor, and --reason are required");
  }

  return withLock(root, async () => {
    const path = claimPath(root, task);
    const claim = JSON.parse(await readFile(path, "utf8"));
    if (
      !isActiveClaim(claim) ||
      !isExpiredClaim(claim, now.getTime()) ||
      !isPastGracePeriod(claim, now.getTime())
    ) {
      throw new Error(`claim ${task} is not past its lease and grace period`);
    }
    const recovered = {
      ...claim,
      status: "recovered",
      recovered_by: actor,
      recovery_reason: reason,
      recovered_at: now.toISOString()
    };
    await writeJsonAtomic(path, recovered);
    return recovered;
  });
}

export async function recoverStaleLock({
  root,
  actor,
  reason,
  now = new Date()
}) {
  if (!actor || !reason) {
    throw new Error("--actor and --reason are required");
  }
  let lockStats;
  let owner;
  try {
    lockStats = await stat(lockDirectory(root));
    owner = JSON.parse(
      await readFile(join(lockDirectory(root), "owner.json"), "utf8")
    );
  } catch (error) {
    if (error?.code === "ENOENT") {
      return { recovered: false, reason: "no lock exists" };
    }
    throw error;
  }
  const ageMs = now.getTime() - lockStats.mtimeMs;
  if (ageMs < STALE_LOCK_MINUTES * 60 * 1000) {
    throw new Error("claim lock is not stale enough for recovery");
  }
  await mkdir(claimsDirectory(root), { recursive: true });
  const audit = {
    schema_version: CLAIM_SCHEMA_VERSION,
    actor,
    reason,
    recovered_at: now.toISOString(),
    lock_age_ms: ageMs,
    previous_owner: owner
  };
  const recoveryPath = `${lockDirectory(root)}.recovery-${randomUUID()}`;
  await rename(lockDirectory(root), recoveryPath);
  audit.recovered_lock_path = recoveryPath;
  await writeJsonAtomic(
    join(claimsDirectory(root), `lock-recovery-${randomUUID()}.json`),
    audit
  );
  await rm(recoveryPath, { recursive: true, force: true });
  return { recovered: true, audit };
}

function parseFlags(args) {
  const flags = {};
  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index];
    if (!argument.startsWith("--")) {
      throw new Error(`unexpected argument: ${argument}`);
    }
    const name = argument.slice(2);
    const value = args[index + 1];
    if (!value || value.startsWith("--")) {
      throw new Error(`--${name} requires a value`);
    }
    if (name === "scope" || name === "dependency") {
      flags[name] ??= [];
      flags[name].push(value);
    } else {
      flags[name] = value;
    }
    index += 1;
  }
  return flags;
}

function usage() {
  return [
    "usage:",
    "  claims.mjs inspect",
    "  claims.mjs claim --task ID --agent ID --branch BRANCH --worktree PATH --scope GLOB [--scope GLOB]",
    "  claims.mjs heartbeat --task ID --agent ID [--lease-minutes N]",
    "  claims.mjs release --task ID --agent ID [--reason TEXT]",
    "  claims.mjs recover --task ID --actor ID --reason TEXT",
    "  claims.mjs recover-lock --actor ID --reason TEXT"
  ].join("\n");
}

async function main(args = process.argv.slice(2)) {
  const [command, ...rawFlags] = args;
  const root = rootFromModule();

  if (command === "inspect") {
    const claims = await readClaimFiles(root);
    console.log(JSON.stringify({ claims }, null, 2));
    return;
  }

  const flags = parseFlags(rawFlags);
  if (command === "claim") {
    const claim = await claimTask({
      root,
      task: flags.task,
      agent: flags.agent,
      branch: flags.branch,
      worktree: flags.worktree,
      writeScope: flags.scope,
      dependencies: flags.dependency ?? [],
      leaseMinutes: Number(flags["lease-minutes"] ?? DEFAULT_LEASE_MINUTES)
    });
    console.log(JSON.stringify(claim, null, 2));
    return;
  }
  if (command === "heartbeat") {
    console.log(
      JSON.stringify(
        await heartbeatTask({
          root,
          task: flags.task,
          agent: flags.agent,
          leaseMinutes: Number(flags["lease-minutes"] ?? DEFAULT_LEASE_MINUTES)
        }),
        null,
        2
      )
    );
    return;
  }
  if (command === "release") {
    console.log(
      JSON.stringify(
        await releaseTask({
          root,
          task: flags.task,
          agent: flags.agent,
          reason: flags.reason
        }),
        null,
        2
      )
    );
    return;
  }
  if (command === "recover") {
    console.log(
      JSON.stringify(
        await recoverExpiredTask({
          root,
          task: flags.task,
          actor: flags.actor,
          reason: flags.reason
        }),
        null,
        2
      )
    );
    return;
  }
  if (command === "recover-lock") {
    console.log(
      JSON.stringify(
        await recoverStaleLock({
          root,
          actor: flags.actor,
          reason: flags.reason
        }),
        null,
        2
      )
    );
    return;
  }

  console.error(usage());
  process.exitCode = 2;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}
