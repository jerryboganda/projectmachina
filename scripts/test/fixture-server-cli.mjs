// Process entry point around `createFixtureServer` (fixture-server.mjs
// stays a pure module, matching its existing convention of only being
// consumed in-process by fixture-server.test.mjs). This CLI wrapper is the
// M2-T02 addition: it lets an out-of-process test harness -- specifically
// `crates/network`'s Rust integration tests, which cannot `import` a JS
// module -- spawn the same fixture routes as a subprocess instead of
// re-implementing them, per the task's "extend the existing loopback
// fixture infrastructure ... rather than building parallel test infra"
// instruction.
//
// Usage: node scripts/test/fixture-server-cli.mjs [--instances=N]
//
// Prints exactly one JSON line to stdout once every instance is listening:
//   {"instances":[{"host":"127.0.0.1","port":51234,"protocol":"http"}, ...]}
// Each instance is bound to its own ephemeral loopback port, so multiple
// instances are genuinely different origins (scheme+host+port), useful for
// cross-origin redirect fixtures without any DNS dependency. The process
// stays alive until it receives SIGTERM/SIGINT or its stdin is closed, then
// stops every server and exits 0.

import { parseArgs } from "node:util";
import { createFixtureServer } from "./fixture-server.mjs";

const { values } = parseArgs({
  options: {
    instances: { type: "string", default: "1" }
  }
});
const instanceCount = Math.max(1, Number.parseInt(values.instances, 10) || 1);

const fixtures = Array.from({ length: instanceCount }, () => createFixtureServer());
const addresses = await Promise.all(fixtures.map((fixture) => fixture.start()));

process.stdout.write(`${JSON.stringify({ instances: addresses })}\n`);

let shuttingDown = false;
async function shutdown() {
  if (shuttingDown) {
    return;
  }
  shuttingDown = true;
  await Promise.all(fixtures.map((fixture) => fixture.stop().catch(() => {})));
  process.exit(0);
}

process.on("SIGTERM", shutdown);
process.on("SIGINT", shutdown);
process.stdin.on("end", shutdown);
process.stdin.resume();
