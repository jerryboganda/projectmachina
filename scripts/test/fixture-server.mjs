import { createHash } from "node:crypto";
import { createServer as createHttpServer } from "node:http";
import { createServer as createHttpsServer } from "node:https";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

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
