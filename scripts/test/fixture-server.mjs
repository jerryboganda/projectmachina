import { createHash } from "node:crypto";
import { gzipSync, deflateSync, brotliCompressSync } from "node:zlib";
import { createServer as createHttpServer } from "node:http";
import { createServer as createHttpsServer } from "node:https";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const root = fileURLToPath(new URL("../..", import.meta.url));
const manifest = JSON.parse(
  await readFile(join(root, "tests/fixtures/manifest.json"), "utf8")
);
const MAX_BODY_BYTES = 64 * 1024;
const WEBSOCKET_MAGIC = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

function isLoopbackHost(host) {
  return host === "127.0.0.1" || host === "::1";
}

function json(response, statusCode, value) {
  const body = JSON.stringify(value);
  response.writeHead(statusCode, {
    "content-type": "application/json; charset=utf-8",
    "content-length": Buffer.byteLength(body),
    "cache-control": "no-store",
    "x-machina-fixture-version": manifest.version
  });
  response.end(body);
}

function html(response, statusCode, body) {
  response.writeHead(statusCode, {
    "content-type": "text/html; charset=utf-8",
    "content-length": Buffer.byteLength(body),
    "cache-control": "no-store",
    "x-machina-fixture-version": manifest.version
  });
  response.end(body);
}

async function readBody(request) {
  const chunks = [];
  let size = 0;
  for await (const chunk of request) {
    size += chunk.length;
    if (size > MAX_BODY_BYTES) {
      throw new Error("fixture request body exceeds limit");
    }
    chunks.push(chunk);
  }
  return Buffer.concat(chunks).toString("utf8");
}

function handleRequest(request, response) {
  const requestUrl = new URL(request.url ?? "/", "http://fixture.invalid");
  const hostHeader = request.headers.host ?? "one.localhost";
  const origin = hostHeader.startsWith("[")
    ? hostHeader.slice(1).split("]")[0]
    : hostHeader.split(":")[0];

  if (requestUrl.pathname === "/health" && request.method === "GET") {
    json(response, 200, {
      fixture_set: manifest.fixture_set,
      version: manifest.version,
      origin,
      external_network: manifest.network.external_network
    });
    return;
  }

  if (requestUrl.pathname === "/origin" && request.method === "GET") {
    json(response, 200, {
      origin,
      fixture_set: manifest.fixture_set,
      external_network: manifest.network.external_network
    });
    return;
  }

  if (requestUrl.pathname === "/redirect" && request.method === "GET") {
    response.writeHead(302, {
      location: "/navigation",
      "cache-control": "no-store"
    });
    response.end();
    return;
  }

  if (requestUrl.pathname === "/navigation" && request.method === "GET") {
    html(
      response,
      200,
      "<!doctype html><html lang=\"en\"><head><title>Machina fixture</title></head><body><main><h1>Navigation fixture</h1><form method=\"post\" action=\"/form\"><label for=\"name\">Name</label><input id=\"name\" name=\"name\" required><button type=\"submit\">Submit</button></form><a href=\"/dom-mutation\">Mutation</a></main></body></html>"
    );
    return;
  }

  if (requestUrl.pathname === "/dom-mutation" && request.method === "GET") {
    html(
      response,
      200,
      "<!doctype html><html lang=\"en\"><body><main id=\"root\"><p data-state=\"initial\">Initial</p></main><script>document.querySelector('[data-state]').textContent='Mutated';</script></body></html>"
    );
    return;
  }

  if (requestUrl.pathname === "/network-policy" && request.method === "GET") {
    json(response, 200, {
      origin,
      redirect_target: "/navigation",
      private_network_allowed: false
    });
    return;
  }

  if (requestUrl.pathname === "/form" && request.method === "POST") {
    readBody(request)
      .then((body) => json(response, 200, { accepted: true, body }))
      .catch((error) => json(response, 413, { accepted: false, error: error.message }));
    return;
  }

  // M2-T02 additions: redirect chains (same-origin decrementing counter,
  // and an optional cross-origin hop via `to=<absolute origin>`), an
  // intentional redirect loop, streaming compression/chunking fixtures,
  // and a slow-trickle route for cancellation/deadline tests. See
  // `.agent-state/design/M2-T02-http-loader-design.md` section 8.
  if (requestUrl.pathname === "/redirect-chain" && request.method === "GET") {
    const remaining = Number.parseInt(requestUrl.searchParams.get("n") ?? "0", 10);
    const crossOriginTarget = requestUrl.searchParams.get("to");
    if (remaining <= 0) {
      json(response, 200, {
        done: true,
        received_authorization: typeof request.headers.authorization === "string",
        received_cookie: typeof request.headers.cookie === "string"
      });
      return;
    }
    const location =
      crossOriginTarget !== null
        ? `${crossOriginTarget}/redirect-chain?n=${remaining - 1}`
        : `/redirect-chain?n=${remaining - 1}`;
    response.writeHead(302, { location, "cache-control": "no-store" });
    response.end();
    return;
  }

  if (requestUrl.pathname === "/redirect-loop" && request.method === "GET") {
    response.writeHead(302, { location: "/redirect-loop", "cache-control": "no-store" });
    response.end();
    return;
  }

  if (requestUrl.pathname.startsWith("/compressed/") && request.method === "GET") {
    const encoding = requestUrl.pathname.slice("/compressed/".length);
    // Deliberately bounded redundancy (each line carries a distinct index)
    // rather than one string repeated verbatim hundreds of times: a
    // near-fully-redundant payload compresses at a ratio a conservative
    // zip-bomb ceiling is specifically designed to reject, which would
    // make this fixture indistinguishable from the abuse case it is not
    // meant to exercise.
    const lines = [];
    for (let index = 0; index < 200; index += 1) {
      lines.push(`line-${index}-machina-fixture-compressible-payload-${manifest.version}`);
    }
    const payload = Buffer.from(lines.join("\n"), "utf8");
    let compressed;
    if (encoding === "gzip") {
      compressed = gzipSync(payload);
    } else if (encoding === "deflate") {
      compressed = deflateSync(payload);
    } else if (encoding === "br") {
      compressed = brotliCompressSync(payload);
    } else {
      json(response, 404, { error: `unknown compression fixture: ${encoding}` });
      return;
    }
    response.writeHead(200, {
      "content-type": "text/plain; charset=utf-8",
      "content-encoding": encoding,
      "content-length": compressed.length,
      "cache-control": "no-store",
      "x-machina-fixture-version": manifest.version,
      "x-machina-decompressed-length": String(payload.length)
    });
    response.end(compressed);
    return;
  }

  if (requestUrl.pathname === "/chunked" && request.method === "GET") {
    response.writeHead(200, {
      "content-type": "text/plain; charset=utf-8",
      "cache-control": "no-store",
      "x-machina-fixture-version": manifest.version
    });
    // No content-length is set, so Node emits Transfer-Encoding: chunked
    // automatically for an HTTP/1.1 response written across multiple
    // `write` calls.
    const chunks = ["first-chunk;", "second-chunk;", "third-chunk;", "fourth-chunk"];
    for (const chunk of chunks) {
      response.write(chunk);
    }
    response.end();
    return;
  }

  if (requestUrl.pathname === "/slow-trickle" && request.method === "GET") {
    const delayMs = Number.parseInt(requestUrl.searchParams.get("delay_ms") ?? "50", 10);
    const chunkCount = Number.parseInt(requestUrl.searchParams.get("chunks") ?? "10", 10);
    response.writeHead(200, {
      "content-type": "text/plain; charset=utf-8",
      "cache-control": "no-store",
      "x-machina-fixture-version": manifest.version
    });
    let sent = 0;
    const timer = setInterval(() => {
      if (sent >= chunkCount || response.destroyed) {
        clearInterval(timer);
        if (!response.destroyed) {
          response.end();
        }
        return;
      }
      response.write(`chunk-${sent};`);
      sent += 1;
    }, delayMs);
    request.on("close", () => clearInterval(timer));
    return;
  }

  json(response, 404, {
    error: "fixture route not found",
    trace_ref: `fixture/${manifest.version}${requestUrl.pathname}`
  });
}

function handleUpgrade(request, socket) {
  const requestUrl = new URL(request.url ?? "/", "http://fixture.invalid");
  const key = request.headers["sec-websocket-key"];
  if (requestUrl.pathname !== manifest.network.websocket_path || typeof key !== "string") {
    socket.destroy();
    return;
  }

  const accept = createHash("sha1")
    .update(`${key}${WEBSOCKET_MAGIC}`)
    .digest("base64");
  socket.write(
    [
      "HTTP/1.1 101 Switching Protocols",
      "Upgrade: websocket",
      "Connection: Upgrade",
      `Sec-WebSocket-Accept: ${accept}`,
      "\r\n"
    ].join("\r\n")
  );
  socket.end();
}

export function createFixtureServer({ host = manifest.network.bind_host, port = 0, tlsOptions } = {}) {
  if (!isLoopbackHost(host)) {
    throw new Error("fixture server may bind only to a loopback host");
  }
  const server = tlsOptions
    ? createHttpsServer(tlsOptions, handleRequest)
    : createHttpServer(handleRequest);
  server.on("upgrade", handleUpgrade);

  return {
    manifest,
    server,
    start() {
      return new Promise((resolve, reject) => {
        const onError = (error) => {
          server.off("listening", onListening);
          reject(error);
        };
        const onListening = () => {
          server.off("error", onError);
          const address = server.address();
          if (!address || typeof address === "string") {
            reject(new Error("fixture server did not expose a socket address"));
            return;
          }
          resolve({
            host,
            port: address.port,
            protocol: tlsOptions ? "https" : "http"
          });
        };
        server.once("error", onError);
        server.once("listening", onListening);
        server.listen(port, host);
      });
    },
    stop() {
      return new Promise((resolve, reject) => {
        server.close((error) => (error ? reject(error) : resolve()));
      });
    }
  };
}
