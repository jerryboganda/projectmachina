import { readdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = fileURLToPath(new URL("../..", import.meta.url));
const COMPLETE_STATES = new Set(["complete", "completed", "done", "merged"]);

function parseDependencies(value) {
  if (!value || value.trim().toLowerCase() === "none") {
    return [];
  }
  const dependencies = new Set(value.match(/M\d+-T\d+/g) ?? []);
  for (const match of value.matchAll(/(M\d+)-T(\d+)\s+through\s+(M\d+)-T(\d+)/gi)) {
    const [, startMilestone, startTask, endMilestone, endTask] = match;
    if (startMilestone !== endMilestone) {
      continue;
    }
    for (let taskNumber = Number(startTask); taskNumber <= Number(endTask); taskNumber += 1) {
      dependencies.add(`${startMilestone}-T${String(taskNumber).padStart(2, "0")}`);
    }
  }
  return [...dependencies].sort((left, right) =>
    left.localeCompare(right, undefined, { numeric: true })
  );
}

export function parseMilestoneTasks(markdown, source) {
  const tasks = [];
  const headings = [...markdown.matchAll(/^## (M\d+-T\d+) [—-] (.+)$/gm)];
  for (const [index, heading] of headings.entries()) {
    const [, id, title] = heading;
    const start = heading.index + heading[0].length;
    const end = headings[index + 1]?.index ?? markdown.length;
    const block = markdown.slice(start, end);
    const role = block.match(/\*\*Primary role:\*\* (.+)/)?.[1]?.trim() ?? "unassigned";
    const dependencyText = block.match(/\*\*Dependencies:\*\* (.+)/)?.[1]?.trim() ?? "none";
    const risk = block.match(/\*\*Risk:\*\* (.+)/)?.[1]?.trim() ?? "unclassified";
    const parallel = block.match(/\*\*May run in parallel:\*\* (.+)/)?.[1]?.trim() ?? "unknown";
    tasks.push({
      id,
      title: title.trim(),
      milestone: id.split("-")[0],
      primary_role: role,
      dependencies: parseDependencies(dependencyText),
      risk,
      may_run_in_parallel: parallel,
      source
    });
  }
  return tasks;
}

function sortTaskIds(left, right) {
  return left.localeCompare(right, undefined, { numeric: true });
}

function registryPath(root) {
  return join(root, ".agent-state", "task-registry.json");
}

function normalizeDependencies(dependencies, taskId) {
  if (!Array.isArray(dependencies)) {
    throw new Error(`task ${taskId} has invalid dependencies`);
  }
  const normalized = [...new Set(dependencies)];
  if (normalized.some((dependency) => typeof dependency !== "string" || dependency.length === 0)) {
    throw new Error(`task ${taskId} has an invalid dependency ID`);
  }
  return normalized.sort(sortTaskIds);
}

function validateRegistryGraph(tasks) {
  const byId = new Map();
  for (const task of tasks) {
    if (!task || typeof task.id !== "string" || task.id.length === 0) {
      throw new Error("task registry contains a task without an ID");
    }
    if (byId.has(task.id)) {
      throw new Error(`duplicate task IDs: ${task.id}`);
    }
    byId.set(task.id, task);
  }

  for (const task of tasks) {
    task.dependencies = normalizeDependencies(task.dependencies ?? [], task.id);
    for (const dependency of task.dependencies) {
      if (!byId.has(dependency)) {
        throw new Error(`task ${task.id} depends on unknown task ${dependency}`);
      }
    }
  }

  const visiting = new Set();
  const visited = new Set();
  function visit(taskId, path = []) {
    if (visiting.has(taskId)) {
      const cycle = [...path, taskId].join(" -> ");
      throw new Error(`task dependency cycle: ${cycle}`);
    }
    if (visited.has(taskId)) {
      return;
    }
    visiting.add(taskId);
    const task = byId.get(taskId);
    for (const dependency of task.dependencies) {
      visit(dependency, [...path, taskId]);
    }
    visiting.delete(taskId);
    visited.add(taskId);
  }
  for (const task of tasks) {
    visit(task.id);
  }
  return byId;
}

export async function loadTaskRegistry(root = repositoryRoot) {
  let parsed;
  try {
    parsed = JSON.parse(await readFile(registryPath(root), "utf8"));
  } catch (error) {
    if (error?.code === "ENOENT") {
      return null;
    }
    throw error;
  }
  if (!parsed || !Array.isArray(parsed.tasks)) {
    throw new Error("task registry must contain a tasks array");
  }
  const tasks = parsed.tasks.map((task) => ({
    ...task,
    dependencies: normalizeDependencies(task.dependencies ?? [], task.id)
  }));
  const byId = validateRegistryGraph(tasks);
  return { ...parsed, tasks, byId };
}

async function loadTaskStatuses(root) {
  try {
    const parsed = JSON.parse(await readFile(join(root, ".agent-state", "task-status.json"), "utf8"));
    const statuses = new Map();
    if (Array.isArray(parsed)) {
      for (const entry of parsed) {
        if (entry?.id && entry.status) {
          statuses.set(entry.id, entry.status);
        }
      }
    } else if (Array.isArray(parsed?.tasks)) {
      for (const entry of parsed.tasks) {
        if (entry?.id && entry.status) {
          statuses.set(entry.id, entry.status);
        }
      }
    } else if (parsed?.tasks && typeof parsed.tasks === "object") {
      for (const [id, status] of Object.entries(parsed.tasks)) {
        if (typeof status === "string") {
          statuses.set(id, status);
        } else if (status?.status) {
          statuses.set(id, status.status);
        }
      }
    }
    return statuses;
  } catch (error) {
    if (error?.code === "ENOENT") {
      return new Map();
    }
    throw error;
  }
}

function sameTaskIds(left, right) {
  const leftIds = [...new Set(left)].sort(sortTaskIds);
  const rightIds = [...new Set(right)].sort(sortTaskIds);
  return (
    leftIds.length === rightIds.length &&
    leftIds.every((dependency, index) => dependency === rightIds[index])
  );
}

/**
 * Validate the task and its dependency declaration before a claim is written.
 *
 * A generated registry is authoritative when present. Dependency completion is
 * checked only when durable task status data is present; the bootstrap registry
 * intentionally contains graph metadata, not mutable execution state.
 */
export async function validateTaskDependencies({
  root = repositoryRoot,
  task,
  dependencies = []
}) {
  if (!task || typeof task !== "string") {
    throw new Error("task ID is required");
  }
  if (!Array.isArray(dependencies)) {
    throw new Error("task dependencies must be an array");
  }
  if (dependencies.some((dependency) => typeof dependency !== "string" || dependency.length === 0)) {
    throw new Error(`task ${task} has an invalid dependency ID`);
  }

  const registry = await loadTaskRegistry(root);
  if (!registry) {
    return {
      registry_available: false,
      task: null,
      dependencies: [...new Set(dependencies)].sort(sortTaskIds)
    };
  }

  const definition = registry.byId.get(task);
  if (!definition) {
    throw new Error(`unknown task ID: ${task}`);
  }
  const declared = definition.dependencies ?? [];
  if (dependencies.length > 0 && !sameTaskIds(dependencies, declared)) {
    throw new Error(
      `dependencies for ${task} do not match the task registry: expected ${declared.join(", ") || "none"}`
    );
  }

  const statuses = await loadTaskStatuses(root);
  const incomplete = declared.filter((dependency) => {
    const dependencyDefinition = registry.byId.get(dependency);
    const status = statuses.get(dependency) ?? dependencyDefinition.status;
    return status !== undefined && !COMPLETE_STATES.has(String(status).toLowerCase());
  });
  if (incomplete.length > 0) {
    throw new Error(`task dependencies are not complete: ${incomplete.join(", ")}`);
  }

  return {
    registry_available: true,
    task: definition,
    dependencies: [...declared].sort(sortTaskIds)
  };
}

export async function generateTaskRegistry(options = {}) {
  const root = typeof options === "string" ? options : options.root ?? repositoryRoot;
  const planningDirectory = join(root, "planning");
  const outputPath = registryPath(root);
  const entries = await readdir(planningDirectory, { withFileTypes: true });
  const tasks = [];
  for (const entry of entries) {
    if (!entry.isFile() || !/^MILESTONE_\d+_.*\.md$/i.test(entry.name)) {
      continue;
    }
    const source = join("planning", entry.name).replaceAll("\\", "/");
    const markdown = await readFile(join(planningDirectory, entry.name), "utf8");
    tasks.push(...parseMilestoneTasks(markdown, source));
  }

  tasks.sort((left, right) => left.id.localeCompare(right.id, undefined, { numeric: true }));
  validateRegistryGraph(tasks);
  if (tasks.length !== 121) {
    throw new Error(`expected 121 milestone tasks, found ${tasks.length}`);
  }

  const registry = {
    schema_version: "0.1.0",
    generated_by: "scripts/agent/task-registry.mjs",
    task_count: tasks.length,
    tasks
  };
  await writeFile(outputPath, `${JSON.stringify(registry, null, 2)}\n`, "utf8");
  return registry;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  generateTaskRegistry({ root: repositoryRoot })
    .then((registry) => console.log(`generated ${registry.task_count} tasks`))
    .catch((error) => {
      console.error(error instanceof Error ? error.message : String(error));
      process.exitCode = 1;
    });
}
