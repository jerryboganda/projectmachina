import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

test("console entrypoint has a document language", async () => {
  const html = await readFile(new URL("../src/app.html", import.meta.url), "utf8");
  assert.match(html, /<html lang="en">/);
});
