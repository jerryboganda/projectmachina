import { constants, realpathSync } from "node:fs";
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
import { spawnSync } from "node:child_process";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { projectClaimEvidence } from "./handoff.mjs";
import { validateTaskDependencies } from "./task-registry.mjs";

export {
  createTaskWorktree,
  createWorktree,
  inspectWorktree,
  inspectWorktrees,
  listWorktrees,
  removeTaskWorktree,
  removeWorktree
} from "./worktree.mjs";

export const CLAIM_SCHEMA_VERSION = 1;
export const DEFAULT_LEASE_MINUTES = 90;
export const DEFAULT_GRACE_MINUTES = 20;
export const STALE_LOCK_MINUTES = 30;
export const STALE_LOCK_MS = STALE_LOCK_MINUTES * 60 * 1000;

const repositoryRoot = fileURLToPath(new URL("../..", import.meta.url));
const PROTECTED_BRANCHES = new Set(["main", "master"]);

function runGit(root, args, { allowFailure = false } = {}) {
  const result = spawnSync("git", args, {
    cwd: root,
    encoding: "utf8",
    shell: false,
    stdio: "pipe"
  });
  if (!allowFailure && (result.error || result.status !== 0)) {
    throw new Error(
      result.stderr?.trim() ||
        result.error?.message ||
        `git failed: git ${args.join(" ")}`
    );
  }
  return result;
}

function gitCommonDirectory(root) {
  const result = runGit(root, ["rev-parse", "--git-common-dir"], {
    allowFailure: true
  });
  if (result.error || result.status !== 0 || !result.stdout.trim()) {
    return null;
  }
  return resolve(root, result.stdout.trim());
}

/**
 * Claims deliberately live beside the common Git metadata so every worktree
 * sees the same lock and records. Non-Git temporary roots are only supported
 * for isolated unit tests and use their local .agent-state directory.
 */
export function getClaimStoreRoot(root = repositoryRoot) {
  const commonDirectory = gitCommonDirectory(root);
  return commonDirectory ? join(commonDirectory, "machina-claims") : join(root, ".agent-state");
}

export function getClaimsDirectory(root = repositoryRoot) {
  return join(getClaimStoreRoot(root), "claims");
}

export const claimStoreRoot = getClaimStoreRoot;

function lockDirectory(root) {
  return join(getClaimStoreRoot(root), "claims.lock");
}

function coordinationLockDirectory(root) {
  return join(getClaimStoreRoot(root), "coordination.lock");
}

function auditDirectory(root) {
  return join(getClaimStoreRoot(root), "audit");
}

export function getClaimLockPath(root = repositoryRoot) {
  return lockDirectory(root);
}

export function getCoordinationLockPath(root = repositoryRoot) {
  return coordinationLockDirectory(root);
}

function safeTaskId(task) {
  if (typeof task !== "string" || task.trim().length === 0) {
    throw new Error("task ID is required");
  }
  const value = task.trim();
  if (!/^[A-Za-z0-9._-]+$/.test(value)) {
    throw new Error(`task ID contains unsupported path characters: ${task}`);
  }
  return value;
}

function claimPath(root, task) {
  return join(getClaimsDirectory(root), `${safeTaskId(task)}.json`);
}

function canonicalPath(root, candidate) {
  const absolute = resolve(root, candidate);
  let real = absolute;
  try {
    real = realpathSync.native(absolute);
  } catch {
    real = absolute;
  }
  return process.platform === "win32" ? real.toLowerCase() : real;
}

function normalizeBranch(branch) {
  if (typeof branch !== "string" || branch.trim().length === 0) {
    throw new Error("branch is required");
  }
  return branch.trim().replace(/^refs\/heads\//, "");
}

function assertProtectedBranch(root, branch) {
  const normalized = normalizeBranch(branch);
  const protectedBranches = new Set(PROTECTED_BRANCHES);
  const defaultHead = runGit(
    root,
    ["symbolic-ref", "--quiet", "--short", "refs/remotes/origin/HEAD"],
    { allowFailure: true }
  );
  if (defaultHead.status === 0 && defaultHead.stdout.trim()) {
    protectedBranches.add(defaultHead.stdout.trim().replace(/^origin\//, "").toLowerCase());
  }
  if (protectedBranches.has(normalized.toLowerCase()) || normalized === "HEAD") {
    throw new Error("active task claims cannot use the protected default branch");
  }
  return normalized;
}

function validateWorktree(root, branch, worktree, allowNonGit) {
  const repository = runGit(root, ["rev-parse", "--show-toplevel"], {
    allowFailure: true
  });
  if (repository.error || repository.status !== 0) {
    if (allowNonGit) {
      return;
    }
    throw new Error("claim worktree validation requires a Git repository");
  }
  const result = runGit(root, ["worktree", "list", "--porcelain"]);
  const expectedPath = canonicalPath(root, worktree);
  const expectedBranch = normalizeBranch(branch);
  const blocks = result.stdout.trim().split(/\r?\n\r?\n/).filter(Boolean);
  const match = blocks.find((block) => {
    const lines = block.split(/\r?\n/);
    const path = lines.find((line) => line.startsWith("worktree "))?.slice(9);
    const actualBranch = lines.find((line) => line.startsWith("branch "))?.slice(7);
    return (
      path &&
      actualBranch === `refs/heads/${expectedBranch}` &&
      canonicalPath(root, path) === expectedPath
    );
  });
  if (!match) {
    throw new Error(`worktree is not registered on the claimed branch: ${worktree}`);
  }
}

function normalizeScopeValue(scope) {
  if (typeof scope !== "string" || scope.trim().length === 0) {
    throw new Error("write scope must be a non-empty path glob");
  }
  const normalized = scope.trim().replaceAll("\\", "/");
  if (
    normalized.includes("\0") ||
    normalized.startsWith("/") ||
    /^[A-Za-z]:/.test(normalized)
  ) {
    throw new Error(`write scope must be repository-relative: ${scope}`);
  }
  const segments = normalized.split("/");
  if (segments.some((segment) => segment === "..")) {
    throw new Error(`write scope escapes repository root: ${scope}`);
  }
  const candidate = segments
    .filter((segment) => segment.length > 0 && segment !== ".")
    .join("/");
  return candidate || ".";
}

export function normalizeScope(scope, root = repositoryRoot) {
  const candidate = normalizeScopeValue(scope);
  if (candidate === ".") {
    return candidate;
  }
  const absolute = resolve(root, candidate);
  const relativePath = relative(resolve(root), absolute).replaceAll("\\", "/");
  if (relativePath.startsWith("../") || relativePath === "..") {
    throw new Error(`write scope escapes repository root: ${scope}`);
  }
  return candidate;
}

function scopeDescriptor(scope) {
  const normalized = normalizeScopeValue(scope);
  const wildcardIndex = normalized.search(/[*?[\]]/);
  if (wildcardIndex === -1) {
    return {
      scope: normalized,
      prefix: normalized,
      wildcard: false,
      segmentBoundary: true
    };
  }
  const rawPrefix = normalized.slice(0, wildcardIndex);
  return {
    scope: normalized,
    prefix: rawPrefix.replace(/\/+$/, ""),
    wildcard: true,
    segmentBoundary: rawPrefix.endsWith("/")
  };
}

function pathPrefixMatches(prefix, candidate) {
  return (
    prefix === candidate ||
    candidate.startsWith(`${prefix}/`) ||
    prefix.startsWith(`${candidate}/`)
  );
}

/**
 * Return true when two canonical repository-relative globs may name the same
 * path. The result is intentionally conservative: a possible intersection
 * blocks a second writer, while unrelated path segments remain available.
 */
export function scopesOverlap(left, right) {
  const leftDescriptor = scopeDescriptor(left);
  const rightDescriptor = scopeDescriptor(right);
  if (leftDescriptor.scope === "." || rightDescriptor.scope === ".") {
    return true;
  }
  const foldCase = process.platform === "win32";
  const leftPrefix = foldCase
    ? leftDescriptor.prefix.toLowerCase()
    : leftDescriptor.prefix;
  const rightPrefix = foldCase
    ? rightDescriptor.prefix.toLowerCase()
    : rightDescriptor.prefix;
  if (!leftPrefix || !rightPrefix) {
    return true;
  }
  if (leftPrefix === rightPrefix) {
    return true;
  }

  const leftBoundary = leftDescriptor.segmentBoundary;
  const rightBoundary = rightDescriptor.segmentBoundary;
  if (
    (leftBoundary && pathPrefixMatches(leftPrefix, rightPrefix)) ||
    (rightBoundary && pathPrefixMatches(rightPrefix, leftPrefix))
  ) {
    return true;
  }

  // A wildcard embedded in a path segment can continue a literal prefix
  // (foo* intersects foobar), but it must not cross an explicit directory
  // boundary (foo/** does not intersect foobar).
  if (!leftBoundary && rightPrefix.startsWith(leftPrefix)) {
    return true;
  }
  if (!rightBoundary && leftPrefix.startsWith(rightPrefix)) {
    return true;
  }
  return false;
}

export function literalPrefix(scope) {
  return scopeDescriptor(scope).prefix;
}

async function writeJsonAtomic(path, value) {
  await mkdir(dirname(path), { recursive: true });
  const temporaryPath = `${path}.${randomUUID()}.tmp`;
  try {
    await writeFile(temporaryPath, `${JSON.stringify(value, null, 2)}\n`, {
      encoding: "utf8",
      flag: "wx"
    });
    try {
      await rename(temporaryPath, path);
    } catch (error) {
      // Windows can reject replacing an existing file with rename. The
      // directory lock makes this fallback exclusive; the temporary file is
      // still never exposed as the claim record.
      if (error?.code !== "EEXIST" && error?.code !== "EPERM") {
        throw error;
      }
      await rm(path, { force: true });
      await rename(temporaryPath, path);
    }
  } finally {
    await rm(temporaryPath, { force: true });
  }
}

async function lockAgeMs(path, now) {
  try {
    return Math.max(0, now.getTime() - (await stat(path)).mtimeMs);
  } catch (error) {
    if (error?.code === "ENOENT") {
      return null;
    }
    throw error;
  }
}

async function acquireDirectoryLock(
  root,
  directory,
  kind,
  { allowStaleRecovery = false, now = new Date() } = {}
) {
  await mkdir(dirname(directory), { recursive: true });
  const lockId = randomUUID();
  const fence = randomUUID();
  try {
    await mkdir(directory);
  } catch (error) {
    if (error?.code !== "EEXIST") {
      throw error;
    }
    const age = await lockAgeMs(directory, now);
    if (age !== null && age >= STALE_LOCK_MS && allowStaleRecovery) {
      const stalePath = `${directory}.stale-${randomUUID()}`;
      await rename(directory, stalePath);
      await rm(stalePath, { recursive: true, force: true });
      const recovered = await acquireDirectoryLock(root, directory, kind, { now });
      return {
        ...recovered,
        stale_lock_recovered: true,
        stale_lock_path: stalePath
      };
    }
    if (age !== null && age >= STALE_LOCK_MS) {
      throw new Error(`${kind} lock is stale; inspect and run recover-lock`);
    }
    throw new Error(`${kind} is locked by another operation`);
  }
  try {
    await writeJsonAtomic(join(directory, "owner.json"), {
      lock_id: lockId,
      fence,
      pid: process.pid,
      acquired_at: new Date().toISOString()
    });
  } catch (error) {
    await rm(directory, { recursive: true, force: true });
    throw error;
  }
  return { lock_id: lockId, fence, directory };
}

async function readLockOwner(directory) {
  return JSON.parse(await readFile(join(directory, "owner.json"), "utf8"));
}

async function assertLockFence(lock) {
  let owner;
  try {
    owner = await readLockOwner(lock.directory);
  } catch (error) {
    throw new Error(`claim lock fence is no longer present: ${error.message}`);
  }
  if (owner.lock_id !== lock.lock_id || owner.fence !== lock.fence) {
    throw new Error("claim lock fence changed; refusing stale operation");
  }
}

async function releaseDirectoryLock(lock, kind) {
  await assertLockFence(lock);
  const releasePath = `${lock.directory}.release-${lock.lock_id}`;
  await rename(lock.directory, releasePath);
  await rm(releasePath, { recursive: true, force: true });
  return { released: true, kind, lock_id: lock.lock_id, fence: lock.fence };
}

async function withClaimLock(root, operation, { now = new Date() } = {}) {
  const coordination = await acquireDirectoryLock(
    root,
    coordinationLockDirectory(root),
    "claim coordination",
    { now }
  );
  let claimLock;
  try {
    claimLock = await acquireDirectoryLock(root, lockDirectory(root), "claim state", {
      now
    });
    try {
      return await operation(claimLock);
    } finally {
      await releaseDirectoryLock(claimLock, "claim state");
    }
  } finally {
    await releaseDirectoryLock(coordination, "claim coordination");
  }
}

async function withCoordinationLock(root, operation, { now, allowStaleRecovery } = {}) {
  const coordination = await acquireDirectoryLock(
    root,
    coordinationLockDirectory(root),
    "claim coordination",
    { now: now ?? new Date(), allowStaleRecovery }
  );
  try {
    return await operation(coordination);
  } finally {
    await releaseDirectoryLock(coordination, "claim coordination");
  }
}

async function readClaimFiles(root) {
  const directory = getClaimsDirectory(root);
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
  return claims.sort((left, right) =>
    String(left.task_id).localeCompare(String(right.task_id), undefined, { numeric: true })
  );
}

export function isActiveClaim(claim) {
  return claim?.status === "active";
}

function parseClaimDate(value, name) {
  const timestamp = Date.parse(value);
  if (!Number.isFinite(timestamp)) {
    throw new Error(`claim has an invalid ${name}`);
  }
  return timestamp;
}

export function isExpiredClaim(claim, now = Date.now()) {
  return parseClaimDate(claim.lease_expires_at, "lease expiry") <= now;
}

export function isPastGracePeriod(claim, now = Date.now()) {
  return parseClaimDate(claim.grace_expires_at, "grace expiry") <= now;
}

function validateIdentity({ task, agent, branch, worktree }) {
  safeTaskId(task);
  for (const [name, value] of [
    ["agent", agent],
    ["branch", branch],
    ["worktree", worktree]
  ]) {
    if (typeof value !== "string" || value.trim().length === 0) {
      throw new Error(`--${name} is required`);
    }
  }
}

function validateLeaseValues(leaseMinutes, graceMinutes) {
  if (!Number.isFinite(leaseMinutes) || leaseMinutes <= 0) {
    throw new Error("lease duration must be a positive number of minutes");
  }
  if (!Number.isFinite(graceMinutes) || graceMinutes < 0) {
    throw new Error("grace duration must be a non-negative number of minutes");
  }
}

function requireInspection(inspection) {
  if (typeof inspection !== "string" || inspection.trim().length === 0) {
    throw new Error("recovery inspection evidence is required");
  }
  return inspection.trim();
}

async function archivePreviousClaim(root, path, task) {
  try {
    const previous = JSON.parse(await readFile(path, "utf8"));
    if (previous.status === "active") {
      throw new Error(`task already has an active claim: ${task}`);
    }
    const history = join(getClaimsDirectory(root), "history");
    await mkdir(history, { recursive: true });
    await rename(
      path,
      join(history, `${safeTaskId(task)}-${Date.now()}-${randomUUID()}.json`)
    );
  } catch (error) {
    if (error?.code !== "ENOENT") {
      throw error;
    }
  }
}

async function projectClaim(claim, root, event) {
  return projectClaimEvidence({
    root,
    claim,
    event,
    generatedAt: event?.at ?? new Date().toISOString()
  });
}

export async function validateKnownTask(root, task) {
  return validateTaskDependencies({ root, task });
}

export async function claimTask({
  root = repositoryRoot,
  task,
  agent,
  branch,
  worktree,
  writeScope,
  dependencies = [],
  contractDependencies = [],
  leaseMinutes = DEFAULT_LEASE_MINUTES,
  graceMinutes = DEFAULT_GRACE_MINUTES,
  allowNonGit = false,
  baseCommit = "unknown",
  currentCommit = "unknown",
  now = new Date()
}) {
  validateIdentity({ task, agent, branch, worktree });
  const normalizedBranch = assertProtectedBranch(root, branch);
  if (canonicalPath(root, ".") === canonicalPath(root, worktree)) {
    throw new Error("active task claims require a separate worktree");
  }
  validateWorktree(root, normalizedBranch, worktree, allowNonGit);
  if (!Array.isArray(writeScope) || writeScope.length === 0) {
    throw new Error("at least one write scope is required");
  }
  if (!Array.isArray(contractDependencies)) {
    throw new Error("contract dependencies must be an array");
  }
  validateLeaseValues(leaseMinutes, graceMinutes);
  const normalizedScopes = [...new Set(writeScope.map((scope) => normalizeScope(scope, root)))].sort();
  const dependencyResult = await validateTaskDependencies({
    root,
    task,
    dependencies
  });
  const claimedAt = now.toISOString();

  const claim = await withClaimLock(
    root,
    async (lock) => {
      await assertLockFence(lock);
      const claims = await readClaimFiles(root);
      const activeClaims = claims.filter(isActiveClaim);
      for (const existing of activeClaims) {
        if (existing.task_id === task) {
          throw new Error(`task already has an active claim: ${task}`);
        }
        if (!Array.isArray(existing.write_scope)) {
          throw new Error(`active claim ${existing.task_id} has invalid write scope data`);
        }
        for (const requestedScope of normalizedScopes) {
          if (existing.write_scope.some((scope) => scopesOverlap(scope, requestedScope))) {
            throw new Error(
              `write scope overlaps active claim ${existing.task_id}: ${requestedScope}`
            );
          }
        }
      }

      const record = {
        schema_version: CLAIM_SCHEMA_VERSION,
        claim_id: randomUUID(),
        owner_token: randomUUID(),
        task_id: safeTaskId(task),
        agent_id: agent.trim(),
        branch: normalizedBranch,
        worktree,
        write_scope: normalizedScopes,
        dependencies: dependencyResult.dependencies,
        contract_dependencies: [...contractDependencies],
        base_commit: baseCommit,
        current_commit: currentCommit,
        lease_minutes: leaseMinutes,
        grace_minutes: graceMinutes,
        claimed_at: claimedAt,
        heartbeat_at: claimedAt,
        lease_expires_at: new Date(
          now.getTime() + leaseMinutes * 60 * 1000
        ).toISOString(),
        grace_expires_at: new Date(
          now.getTime() + (leaseMinutes + graceMinutes) * 60 * 1000
        ).toISOString(),
        status: "active",
        lock_fence: lock.fence
      };
      const path = claimPath(root, task);
      await archivePreviousClaim(root, path, task);
      await assertLockFence(lock);
      await writeJsonAtomic(path, record);
      await assertLockFence(lock);
      await projectClaim(record, root, {
        type: "claim",
        status: "active",
        at: claimedAt
      });
      return record;
    },
    { now }
  );
  return claim;
}

function validateClaimOwner(root, claim, agent, branch, worktree, allowNonGit) {
  if (!agent || claim.agent_id !== agent) {
    throw new Error(`claim ${claim.task_id} belongs to another agent`);
  }
  if (!branch || !worktree) {
    throw new Error("--branch and --worktree are required for claim mutation");
  }
  const normalizedBranch = assertProtectedBranch(root, branch);
  if (claim.branch !== normalizedBranch) {
    throw new Error("claim branch does not match the caller branch");
  }
  if (canonicalPath(root, claim.worktree) !== canonicalPath(root, worktree)) {
    throw new Error("claim worktree does not match the caller worktree");
  }
  if (canonicalPath(root, ".") === canonicalPath(root, worktree)) {
    throw new Error("claim mutation requires a separate worktree");
  }
  validateWorktree(root, normalizedBranch, worktree, allowNonGit);
  return normalizedBranch;
}

function validateOwnerToken(claim, ownerToken) {
  if (typeof ownerToken !== "string" || ownerToken.length === 0) {
    throw new Error("owner token is required for claim mutation");
  }
  if (ownerToken !== claim.owner_token) {
    throw new Error("claim owner token does not match");
  }
}

async function readClaimForMutation(root, task, agent, branch, worktree, allowNonGit) {
  const path = claimPath(root, task);
  const claim = JSON.parse(await readFile(path, "utf8"));
  if (!isActiveClaim(claim)) {
    throw new Error(`claim ${task} is not active`);
  }
  validateClaimOwner(root, claim, agent, branch, worktree, allowNonGit);
  return { claim, path };
}

export async function heartbeatTask({
  root = repositoryRoot,
  task,
  agent,
  branch,
  worktree,
  ownerToken,
  leaseMinutes,
  allowNonGit = false,
  now = new Date()
}) {
  if (!task || !agent) {
    throw new Error("--task and --agent are required");
  }
  const updated = await withClaimLock(
    root,
    async (lock) => {
      const { claim, path } = await readClaimForMutation(
        root,
        task,
        agent,
        branch,
        worktree,
        allowNonGit
      );
      validateOwnerToken(claim, ownerToken);
      await assertLockFence(lock);
      if (isExpiredClaim(claim, now.getTime())) {
        throw new Error(`claim ${task} is expired; recover it after the grace period`);
      }
      const duration = leaseMinutes ?? claim.lease_minutes ?? DEFAULT_LEASE_MINUTES;
      const grace = claim.grace_minutes ?? DEFAULT_GRACE_MINUTES;
      validateLeaseValues(duration, grace);
      const heartbeatAt = now.toISOString();
      if (Date.parse(heartbeatAt) < Date.parse(claim.heartbeat_at)) {
        throw new Error("heartbeat timestamp cannot move backwards");
      }
      const next = {
        ...claim,
        lease_minutes: duration,
        grace_minutes: grace,
        heartbeat_at: heartbeatAt,
        lease_expires_at: new Date(
          now.getTime() + duration * 60 * 1000
        ).toISOString(),
        grace_expires_at: new Date(
          now.getTime() + (duration + grace) * 60 * 1000
        ).toISOString(),
        lock_fence: lock.fence
      };
      await writeJsonAtomic(path, next);
      await assertLockFence(lock);
      await projectClaim(next, root, {
        type: "heartbeat",
        status: "active",
        at: next.heartbeat_at
      });
      return next;
    },
    { now }
  );
  return updated;
}

export async function releaseTask({
  root = repositoryRoot,
  task,
  agent,
  branch,
  worktree,
  ownerToken,
  reason = "completed",
  allowNonGit = false,
  now = new Date()
}) {
  if (!task || !agent) {
    throw new Error("--task and --agent are required");
  }
  if (typeof reason !== "string" || reason.trim().length === 0) {
    throw new Error("release reason is required");
  }
  const released = await withClaimLock(
    root,
    async (lock) => {
      const { claim, path } = await readClaimForMutation(
        root,
        task,
        agent,
        branch,
        worktree,
        allowNonGit
      );
      validateOwnerToken(claim, ownerToken);
      await assertLockFence(lock);
      if (isExpiredClaim(claim, now.getTime())) {
        throw new Error(`claim ${task} is expired; recover it after the grace period`);
      }
      const next = {
        ...claim,
        status: "released",
        release_reason: reason.trim(),
        released_at: now.toISOString(),
        lock_fence: lock.fence
      };
      await writeJsonAtomic(path, next);
      await assertLockFence(lock);
      await projectClaim(next, root, {
        type: "release",
        status: "released",
        at: next.released_at,
        reason: next.release_reason
      });
      return next;
    },
    { now }
  );
  return released;
}

export async function recoverExpiredTask({
  root = repositoryRoot,
  task,
  actor,
  reason,
  inspection,
  now = new Date()
}) {
  if (!task || !actor || !reason) {
    throw new Error("--task, --actor, and --reason are required");
  }
  const inspectionEvidence = requireInspection(inspection);
  const recovered = await withClaimLock(
    root,
    async (lock) => {
      const path = claimPath(root, task);
      const claim = JSON.parse(await readFile(path, "utf8"));
      if (
        !isActiveClaim(claim) ||
        !isExpiredClaim(claim, now.getTime()) ||
        !isPastGracePeriod(claim, now.getTime())
      ) {
        throw new Error(`claim ${task} is not past its lease and grace period`);
      }
      await assertLockFence(lock);
      const next = {
        ...claim,
        status: "recovered",
        recovered_by: actor,
        recovery_reason: reason,
        recovery_inspection: inspectionEvidence,
        recovered_at: now.toISOString(),
        lock_fence: lock.fence
      };
      await writeJsonAtomic(path, next);
      await assertLockFence(lock);
      await projectClaim(next, root, {
        type: "recovery",
        status: "recovered",
        at: next.recovered_at,
        actor,
        reason
      });
      return next;
    },
    { now }
  );
  return recovered;
}

export async function recoverStaleLock({
  root = repositoryRoot,
  actor,
  reason,
  inspection,
  now = new Date()
}) {
  if (!actor || !reason) {
    throw new Error("--actor and --reason are required");
  }
  const inspectionEvidence = requireInspection(inspection);
  return withCoordinationLock(
    root,
    async (coordination) => {
      const directory = lockDirectory(root);
      const ageMs = await lockAgeMs(directory, now);
      if (ageMs === null) {
        if (coordination.stale_lock_recovered) {
          const coordinationAudit = {
            schema_version: CLAIM_SCHEMA_VERSION,
            type: "stale-coordination-lock-recovery",
            actor,
            reason,
            inspection: inspectionEvidence,
            recovered_at: now.toISOString(),
            recovered_lock_path: coordination.stale_lock_path,
            fencing_token: randomUUID(),
            coordination_fence: coordination.fence
          };
          await mkdir(auditDirectory(root), { recursive: true });
          await writeJsonAtomic(
            join(auditDirectory(root), `lock-recovery-${randomUUID()}.json`),
            coordinationAudit
          );
        }
        return {
          recovered: false,
          coordination_fence: coordination.fence,
          coordination_lock_recovered: coordination.stale_lock_recovered ?? false,
          reason: "no lock exists"
        };
      }
      if (ageMs < STALE_LOCK_MS) {
        throw new Error("claim lock is not stale enough for recovery");
      }
      let previousOwner = { status: "unknown-owner-metadata" };
      try {
        previousOwner = await readLockOwner(directory);
      } catch (error) {
        if (error?.code !== "ENOENT") {
          throw error;
        }
      }
      const recoveredPath = `${directory}.recovery-${randomUUID()}`;
      await rename(directory, recoveredPath);
      const audit = {
        schema_version: CLAIM_SCHEMA_VERSION,
        type: "stale-lock-recovery",
        actor,
        reason,
        inspection: inspectionEvidence,
        recovered_at: now.toISOString(),
        lock_age_ms: ageMs,
        previous_owner: previousOwner,
        recovered_lock_path: recoveredPath,
        fencing_token: randomUUID(),
        coordination_fence: coordination.fence,
        coordination_lock_recovered: coordination.stale_lock_recovered ?? false
      };
      await mkdir(auditDirectory(root), { recursive: true });
      await writeJsonAtomic(
        join(auditDirectory(root), `lock-recovery-${randomUUID()}.json`),
        audit
      );
      await rm(recoveredPath, { recursive: true, force: true });
      return { recovered: true, audit };
    },
    { now, allowStaleRecovery: true }
  );
}

export async function inspectClaims({ root = repositoryRoot, now = new Date() } = {}) {
  const claims = await readClaimFiles(root);
  const lockPath = lockDirectory(root);
  const coordinationPath = coordinationLockDirectory(root);
  return {
    schema_version: CLAIM_SCHEMA_VERSION,
    store_root: getClaimStoreRoot(root),
    claims: claims.map((claim) => ({
      ...claim,
      expired: isActiveClaim(claim) ? isExpiredClaim(claim, now.getTime()) : false,
      past_grace: isActiveClaim(claim)
        ? isPastGracePeriod(claim, now.getTime())
        : false
    })),
    locks: {
      claim: {
        path: lockPath,
        exists: (await lockAgeMs(lockPath, now)) !== null,
        age_ms: await lockAgeMs(lockPath, now)
      },
      coordination: {
        path: coordinationPath,
        exists: (await lockAgeMs(coordinationPath, now)) !== null,
        age_ms: await lockAgeMs(coordinationPath, now)
      }
    }
  };
}

function parseFlags(args) {
  const flags = {};
  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index];
    if (!argument.startsWith("--")) {
      throw new Error(`unexpected argument: ${argument}`);
    }
    const name = argument.slice(2);
    if (name === "allow-non-git") {
      flags[name] = true;
      continue;
    }
    const value = args[index + 1];
    if (!value || value.startsWith("--")) {
      throw new Error(`--${name} requires a value`);
    }
    if (name === "scope" || name === "dependency" || name === "contract-dependency") {
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
    "  claims.mjs heartbeat --task ID --agent ID --branch BRANCH --worktree PATH [--lease-minutes N]",
    "  claims.mjs release --task ID --agent ID --branch BRANCH --worktree PATH [--reason TEXT]",
    "  claims.mjs recover --task ID --actor ID --reason TEXT [--inspection TEXT]",
    "  claims.mjs recover-lock --actor ID --reason TEXT [--inspection TEXT]"
  ].join("\n");
}

async function main(args = process.argv.slice(2)) {
  const [command, ...rawFlags] = args;
  const root = repositoryRoot;

  if (command === "inspect") {
    console.log(JSON.stringify(await inspectClaims({ root }), null, 2));
    return;
  }

  const flags = parseFlags(rawFlags);
  if (command === "claim") {
    console.log(
      JSON.stringify(
        await claimTask({
          root,
          task: flags.task,
          agent: flags.agent,
          branch: flags.branch,
          worktree: flags.worktree,
          writeScope: flags.scope,
          dependencies: flags.dependency ?? [],
          contractDependencies: flags["contract-dependency"] ?? [],
          leaseMinutes: Number(flags["lease-minutes"] ?? DEFAULT_LEASE_MINUTES),
          graceMinutes: Number(flags["grace-minutes"] ?? DEFAULT_GRACE_MINUTES),
          allowNonGit: flags["allow-non-git"],
          baseCommit: flags["base-commit"] ?? "unknown",
          currentCommit: flags["current-commit"] ?? "unknown"
        }),
        null,
        2
      )
    );
    return;
  }
  if (command === "heartbeat") {
    console.log(
      JSON.stringify(
        await heartbeatTask({
          root,
          task: flags.task,
          agent: flags.agent,
          branch: flags.branch,
          worktree: flags.worktree,
          ownerToken: flags["owner-token"],
          leaseMinutes:
            flags["lease-minutes"] === undefined
              ? undefined
              : Number(flags["lease-minutes"]),
          allowNonGit: flags["allow-non-git"]
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
          branch: flags.branch,
          worktree: flags.worktree,
          ownerToken: flags["owner-token"],
          reason: flags.reason,
          allowNonGit: flags["allow-non-git"]
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
          reason: flags.reason,
          inspection: flags.inspection
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
          reason: flags.reason,
          inspection: flags.inspection
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
