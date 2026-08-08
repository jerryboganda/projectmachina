import { readdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../..", import.meta.url));
const planningDirectory = join(root, "planning");
const outputPath = join(root, ".agent-state", "task-registry.json");

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

export async function generateTaskRegistry() {
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
  const duplicateIds = tasks
    .map((task) => task.id)
    .filter((id, index, ids) => ids.indexOf(id) !== index);
  if (duplicateIds.length > 0) {
    throw new Error(`duplicate task IDs: ${[...new Set(duplicateIds)].join(", ")}`);
  }
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
  generateTaskRegistry()
    .then((registry) => console.log(`generated ${registry.task_count} tasks`))
    .catch((error) => {
      console.error(error instanceof Error ? error.message : String(error));
      process.exitCode = 1;
    });
}
