import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../..", import.meta.url));
const model = JSON.parse(
  await readFile(join(root, "security/requirements.json"), "utf8")
);
const registry = JSON.parse(
  await readFile(join(root, ".agent-state/task-registry.json"), "utf8")
);
const expected = new Set(Array.from({ length: 18 }, (_, index) => `TM-${String(index + 1).padStart(2, "0")}`));
const actual = new Set();
const failures = [];
const taskIds = new Set(registry.tasks.map((task) => task.id));

for (const requirement of model.requirements) {
  if (actual.has(requirement.id)) {
    failures.push(`duplicate requirement: ${requirement.id}`);
  }
  actual.add(requirement.id);
  if (
    !requirement.owner ||
    !Array.isArray(requirement.tasks) ||
    requirement.tasks.length === 0 ||
    !Array.isArray(requirement.test_tags) ||
    requirement.test_tags.length === 0
  ) {
    failures.push(`incomplete traceability: ${requirement.id}`);
  }
  for (const task of requirement.tasks ?? []) {
    if (!taskIds.has(task)) {
      failures.push(`${requirement.id} references unknown task: ${task}`);
    }
  }
}

for (const id of expected) {
  if (!actual.has(id)) {
    failures.push(`missing threat requirement: ${id}`);
  }
}

if (failures.length > 0) {
  console.error(failures.join("\n"));
  process.exit(1);
}
console.log("security requirement traceability check: passed");
