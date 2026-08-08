import { access, mkdir, stat } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { realpathSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const repositoryRoot = fileURLToPath(new URL("../..", import.meta.url));
const PROTECTED_BRANCHES = new Set(["main", "master"]);

function canonicalPath(root, candidate) {
  const absolute = resolve(root, candidate);
  try {
    return process.platform === "win32"
      ? realpathSync.native(absolute).toLowerCase()
      : realpathSync.native(absolute);
  } catch {
    return process.platform === "win32" ? absolute.toLowerCase() : absolute;
  }
}

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

function requireString(value, name) {
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error(`${name} is required`);
  }
  return value.trim();
}

function normalizeBranch(branch) {
  const value = requireString(branch, "branch").replace(/^refs\/heads\//, "");
  if (value.includes("..") || value.endsWith("/") || value.startsWith("-")) {
    throw new Error(`invalid branch name: ${branch}`);
  }
  return value;
}

function assertTaskBranch(branch) {
  const normalized = normalizeBranch(branch);
  if (PROTECTED_BRANCHES.has(normalized.toLowerCase())) {
    throw new Error("worktree operations cannot mutate the protected default branch");
  }
  return normalized;
}

function assertRepository(root) {
  runGit(root, ["rev-parse", "--show-toplevel"]);
}

function parseWorktreeList(output) {
  const worktrees = [];
  const blocks = output.trim().split(/\r?\n\r?\n/).filter(Boolean);
  for (const block of blocks) {
    const lines = block.split(/\r?\n/);
    const path = lines.find((line) => line.startsWith("worktree "))?.slice(9);
    if (!path) {
      continue;
    }
    const head = lines.find((line) => line.startsWith("HEAD "))?.slice(5);
    const branchLine = lines.find((line) => line.startsWith("branch "))?.slice(7);
    worktrees.push({
      path,
      canonical_path: canonicalPath(path, "."),
      head: head ?? null,
      branch: branchLine?.replace(/^refs\/heads\//, "") ?? null,
      detached: lines.includes("detached"),
      locked: lines.some((line) => line.startsWith("locked")),
      prunable: lines.some((line) => line.startsWith("prunable"))
    });
  }
  return worktrees;
}

export function listWorktrees(root = repositoryRoot) {
  assertRepository(root);
  return parseWorktreeList(runGit(root, ["worktree", "list", "--porcelain"]).stdout);
}

function findWorktree(root, worktrees, worktree, branch) {
  const expectedPath = worktree ? canonicalPath(root, worktree) : null;
  const expectedBranch = branch ? normalizeBranch(branch) : null;
  return worktrees.find((entry) => {
    const pathMatches = !expectedPath || entry.canonical_path === expectedPath;
    const branchMatches = !expectedBranch || entry.branch === expectedBranch;
    return pathMatches && branchMatches;
  });
}

export function inspectWorktree(
  { root = repositoryRoot, worktree, branch } = {},
  legacyWorktree,
  legacyBranch
) {
  if (typeof arguments[0] === "string") {
    root = arguments[0];
    worktree = legacyWorktree;
    branch = legacyBranch;
  }
  const worktrees = listWorktrees(root);
  if (!worktree && !branch) {
    return { root, worktrees };
  }
  const entry = findWorktree(root, worktrees, worktree, branch);
  if (!entry) {
    throw new Error(
      `registered worktree not found: ${worktree ?? `branch ${branch}`}`
    );
  }
  const status = runGit(entry.path, ["status", "--porcelain", "--untracked-files=all"]);
  return {
    root,
    worktree: entry,
    clean: status.stdout.trim().length === 0,
    status: status.stdout
  };
}

async function pathExists(path) {
  try {
    await access(path);
    return true;
  } catch (error) {
    if (error?.code === "ENOENT") {
      return false;
    }
    throw error;
  }
}

export async function createTaskWorktree({
  root = repositoryRoot,
  task,
  branch,
  worktree,
  baseCommit,
  checkout = true
}) {
  requireString(task, "task ID");
  const normalizedBranch = assertTaskBranch(branch);
  const target = requireString(worktree, "worktree");
  if (target.startsWith("-")) {
    throw new Error("worktree path cannot start with a Git option");
  }
  if (canonicalPath(root, ".") === canonicalPath(root, target)) {
    throw new Error("task worktree must be separate from the repository checkout");
  }
  assertRepository(root);
  const existing = listWorktrees(root);
  if (findWorktree(root, existing, target)) {
    throw new Error(`worktree is already registered: ${target}`);
  }
  const targetAbsolute = resolve(root, target);
  if (await pathExists(targetAbsolute)) {
    let metadata;
    try {
      metadata = await stat(targetAbsolute);
    } catch {
      metadata = null;
    }
    throw new Error(
      metadata?.isDirectory()
        ? `worktree path already exists and is not registered: ${target}`
        : `worktree path is already occupied: ${target}`
    );
  }
  await mkdir(dirname(targetAbsolute), { recursive: true });

  const branchRef = runGit(
    root,
    ["show-ref", "--verify", "--quiet", `refs/heads/${normalizedBranch}`],
    { allowFailure: true }
  );
  const commit = baseCommit ?? runGit(root, ["rev-parse", "HEAD"]).stdout.trim();
  if (!commit || commit.startsWith("-")) {
    throw new Error("base commit must be a Git commit reference");
  }
  runGit(root, ["rev-parse", "--verify", `${commit}^{commit}`]);

  const args = ["worktree", "add"];
  if (!checkout) {
    args.push("--no-checkout");
  }
  if (branchRef.status === 0) {
    args.push(target, normalizedBranch);
  } else {
    args.push("-b", normalizedBranch, target, commit);
  }
  runGit(root, args);
  return inspectWorktree({ root, worktree: target, branch: normalizedBranch });
}

export async function removeTaskWorktree({
  root = repositoryRoot,
  worktree,
  branch,
  force = false,
  deleteBranch = false
}) {
  const target = requireString(worktree, "worktree");
  if (target.startsWith("-")) {
    throw new Error("worktree path cannot start with a Git option");
  }
  assertRepository(root);
  const inspected = inspectWorktree({ root, worktree: target, branch });
  const actualBranch = inspected.worktree.branch;
  if (actualBranch) {
    assertTaskBranch(actualBranch);
  }
  if (!inspected.clean && !force) {
    throw new Error("worktree has uncommitted changes; pass force explicitly to remove it");
  }
  const args = ["worktree", "remove"];
  if (force) {
    args.push("--force");
  }
  args.push(target);
  runGit(root, args);
  let branchDeleted = false;
  if (deleteBranch && actualBranch) {
    runGit(root, ["branch", force ? "-D" : "-d", actualBranch]);
    branchDeleted = true;
  }
  return {
    removed: true,
    worktree: target,
    branch: actualBranch,
    branch_deleted: branchDeleted
  };
}

export const createWorktree = createTaskWorktree;
export const inspectWorktrees = listWorktrees;
export const removeWorktree = removeTaskWorktree;

function parseFlags(args) {
  const flags = {};
  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index];
    if (!argument.startsWith("--")) {
      throw new Error(`unexpected argument: ${argument}`);
    }
    const name = argument.slice(2);
    if (name === "force" || name === "no-checkout" || name === "delete-branch") {
      flags[name] = true;
      continue;
    }
    const value = args[index + 1];
    if (!value || value.startsWith("--")) {
      throw new Error(`--${name} requires a value`);
    }
    flags[name] = value;
    index += 1;
  }
  return flags;
}

function usage() {
  return [
    "usage:",
    "  worktree.mjs inspect [--worktree PATH] [--branch BRANCH]",
    "  worktree.mjs create --task ID --branch BRANCH --worktree PATH [--base-commit SHA]",
    "  worktree.mjs remove --worktree PATH [--branch BRANCH] [--force] [--delete-branch]"
  ].join("\n");
}

async function main(args = process.argv.slice(2)) {
  const [command, ...rawFlags] = args;
  const flags = parseFlags(rawFlags);
  if (command === "inspect") {
    console.log(
      JSON.stringify(
        inspectWorktree({
          root: repositoryRoot,
          worktree: flags.worktree,
          branch: flags.branch
        }),
        null,
        2
      )
    );
    return;
  }
  if (command === "create") {
    console.log(
      JSON.stringify(
        await createTaskWorktree({
          root: repositoryRoot,
          task: flags.task,
          branch: flags.branch,
          worktree: flags.worktree,
          baseCommit: flags["base-commit"],
          checkout: !flags["no-checkout"]
        }),
        null,
        2
      )
    );
    return;
  }
  if (command === "remove") {
    console.log(
      JSON.stringify(
        await removeTaskWorktree({
          root: repositoryRoot,
          worktree: flags.worktree,
          branch: flags.branch,
          force: flags.force,
          deleteBranch: flags["delete-branch"]
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
