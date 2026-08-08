import { rm } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../..", import.meta.url));
const paths = ["target", "build", "apps/console/.svelte-kit", "apps/console/node_modules", "node_modules"];

for (const relativePath of paths) {
  await rm(join(root, relativePath), { recursive: true, force: true });
}

console.log("Project Machina clean: complete");
