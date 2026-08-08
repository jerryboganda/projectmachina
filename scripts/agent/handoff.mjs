import { randomUUID } from "node:crypto";
import { mkdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";

export const HANDOFF_SCHEMA_VERSION = "0.1.0";
export const EVIDENCE_PROJECTION_SCHEMA_VERSION = "0.1.0";

function requiredString(value, name) {
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error(`${name} is required`);
  }
  return value.trim();
}

function safeFileStem(value) {
  const stem = requiredString(value, "task ID").replace(/[^A-Za-z0-9._-]/g, "_");
  if (stem === "." || stem === "..") {
    throw new Error("task ID is not a valid evidence name");
  }
  return stem;
}

function arrayValue(value) {
  if (value === undefined) {
    return [];
  }
  if (!Array.isArray(value)) {
    throw new Error("evidence list fields must be arrays");
  }
  return value;
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

function evidenceDirectory(root) {
  return join(root, ".agent-state", "evidence");
}

function identityFrom(options) {
  return {
    task: requiredString(options.taskId ?? options.task_id, "task ID"),
    agent: requiredString(options.agent ?? options.agent_id, "agent"),
    branch: requiredString(options.branch, "branch"),
    worktree: requiredString(options.worktree, "worktree"),
    base_commit: options.baseCommit ?? options.base_commit ?? "unknown",
    current_commit: options.currentCommit ?? options.current_commit ?? "unknown"
  };
}

function renderList(items, empty = "None recorded.") {
  if (items.length === 0) {
    return empty;
  }
  return items
    .map((item) => {
      if (typeof item === "string") {
        return `- ${item}`;
      }
      return `- \`${JSON.stringify(item)}\``;
    })
    .join("\n");
}

function renderHandoffMarkdown(record) {
  const identity = record.identity;
  return [
    `# Handoff — ${record.task_id}`,
    "",
    "## Identity",
    `- Task: ${record.task_id}`,
    `- Agent/tool: ${record.agent_id}`,
    `- Branch: ${identity.branch}`,
    `- Worktree: ${identity.worktree}`,
    `- Base commit: ${identity.base_commit}`,
    `- Current commit: ${identity.current_commit}`,
    `- Claim/lease: \`${JSON.stringify(record.claim ?? null)}\``,
    "",
    "## Objective and acceptance criteria",
    record.objective || "Not recorded.",
    "",
    renderList(record.acceptance_criteria),
    "",
    "## Completed",
    renderList(record.completed),
    "",
    "## In progress",
    renderList(record.in_progress),
    "",
    "## Decisions and invariants",
    renderList(record.decisions),
    "",
    "## Commands and results",
    renderList(record.commands),
    "",
    "## Failures and reproductions",
    renderList(record.failures),
    "",
    "## Changed files",
    renderList(record.changed_files),
    "",
    "## Remaining steps",
    renderList(record.remaining_steps),
    "",
    "## Risks and blockers",
    renderList(record.risks),
    "",
    "## Recommended next action",
    record.recommended_next_action || "No next action recorded.",
    ""
  ].join("\n");
}

/**
 * Write both machine-readable and human-readable resumable handoff records.
 */
export async function writeHandoff(options) {
  const identity = identityFrom(options);
  const taskId = identity.task;
  const stem = safeFileStem(taskId);
  const generatedAt = options.generatedAt ?? new Date().toISOString();
  const record = {
    schema_version: HANDOFF_SCHEMA_VERSION,
    type: "handoff",
    task_id: taskId,
    agent_id: identity.agent,
    identity,
    generated_at: generatedAt,
    objective: options.objective ?? "",
    acceptance_criteria: arrayValue(options.acceptanceCriteria ?? options.acceptance_criteria),
    claim: options.claim ?? null,
    completed: arrayValue(options.completed),
    in_progress: arrayValue(options.inProgress ?? options.in_progress),
    decisions: arrayValue(options.decisions),
    commands: arrayValue(options.commands),
    failures: arrayValue(options.failures),
    changed_files: arrayValue(options.changedFiles ?? options.changed_files),
    remaining_steps: arrayValue(options.remainingSteps ?? options.remaining_steps),
    risks: arrayValue(options.risks),
    artifacts: arrayValue(options.artifacts),
    recommended_next_action: options.recommendedNextAction ?? options.recommended_next_action ?? ""
  };
  const directory = evidenceDirectory(options.root);
  const jsonPath = join(directory, `${stem}.handoff.json`);
  const markdownPath = join(directory, `${stem}.handoff.md`);
  await writeJsonAtomic(jsonPath, record);
  await mkdir(dirname(markdownPath), { recursive: true });
  await writeFile(markdownPath, renderHandoffMarkdown(record), "utf8");
  return { record, json_path: jsonPath, markdown_path: markdownPath };
}

export async function readHandoff({ root, taskId }) {
  const path = join(evidenceDirectory(root), `${safeFileStem(taskId)}.handoff.json`);
  return JSON.parse(await readFile(path, "utf8"));
}

function renderEvidenceMarkdown(record) {
  const identity = record.identity;
  return [
    `# Evidence — ${record.task_id}`,
    "",
    "## Identity",
    `- Task: ${record.task_id}`,
    `- Status: \`${record.status}\``,
    `- Agent: ${record.agent_id}`,
    `- Branch: ${identity.branch}`,
    `- Worktree: ${identity.worktree}`,
    `- Base commit: ${identity.base_commit}`,
    `- Current commit: ${identity.current_commit}`,
    "",
    "## Acceptance",
    renderList(record.acceptance_criteria),
    "",
    "## Commands and results",
    renderList(record.commands),
    "",
    "## Artifacts",
    renderList(record.artifacts),
    "",
    "## Changed files",
    renderList(record.changed_files),
    "",
    "## Decisions",
    renderList(record.decisions),
    "",
    "## Failures and reproductions",
    renderList(record.failures),
    "",
    "## Risks and blockers",
    renderList(record.risks),
    "",
    "## Next action",
    record.next_action || "No next action recorded.",
    ""
  ].join("\n");
}

/**
 * Project task state into the tracked .agent-state/evidence directory.
 */
export async function writeEvidenceProjection(options) {
  const identity = identityFrom(options);
  const taskId = identity.task;
  const stem = safeFileStem(options.fileStem ?? taskId);
  const record = {
    schema_version: EVIDENCE_PROJECTION_SCHEMA_VERSION,
    type: "task-evidence",
    task_id: taskId,
    status: options.status ?? "in-progress",
    agent_id: identity.agent,
    identity,
    generated_at: options.generatedAt ?? new Date().toISOString(),
    objective: options.objective ?? "",
    acceptance_criteria: arrayValue(options.acceptanceCriteria ?? options.acceptance_criteria),
    claim: options.claim ?? null,
    commands: arrayValue(options.commands),
    artifacts: arrayValue(options.artifacts),
    changed_files: arrayValue(options.changedFiles ?? options.changed_files),
    decisions: arrayValue(options.decisions),
    failures: arrayValue(options.failures),
    risks: arrayValue(options.risks),
    next_action: options.nextAction ?? options.next_action ?? ""
  };
  const directory = evidenceDirectory(options.root);
  const jsonPath = join(directory, `${stem}.json`);
  const markdownPath = join(directory, `${stem}.md`);
  await writeJsonAtomic(jsonPath, record);
  await mkdir(dirname(markdownPath), { recursive: true });
  await writeFile(markdownPath, renderEvidenceMarkdown(record), "utf8");
  return { record, json_path: jsonPath, markdown_path: markdownPath };
}

/**
 * Keep claim transitions durable without overwriting the task's main report.
 */
export async function projectClaimEvidence({ root, claim, event, generatedAt }) {
  if (!claim?.task_id) {
    throw new Error("claim.task_id is required");
  }
  return writeEvidenceProjection({
    root,
    taskId: claim.task_id,
    agent: claim.agent_id,
    branch: claim.branch,
    worktree: claim.worktree,
    baseCommit: claim.base_commit ?? "unknown",
    currentCommit: claim.current_commit ?? "unknown",
    fileStem: `${safeFileStem(claim.task_id)}.claim`,
    status: claim.status,
    claim,
    generatedAt,
    commands: event ? [event] : [],
    changedFiles: claim.write_scope ?? [],
    nextAction: claim.status === "active" ? "Continue work and heartbeat before the lease expires." : ""
  });
}

export const createHandoff = writeHandoff;
export const projectTaskEvidence = writeEvidenceProjection;
