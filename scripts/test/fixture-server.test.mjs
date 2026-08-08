import assert from "node:assert/strict";
import http from "node:http";
import https from "node:https";
import { existsSync } from "node:fs";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import net from "node:net";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { createFixtureServer } from "./fixture-server.mjs";

function requestOrigin(address, origin) {
  return new Promise((resolve, reject) => {
    const request = http.request(
      {
        hostname: address.host,
        port: address.port,
        path: "/origin",
        headers: { host: `${origin}:${address.port}` }
      },
      (response) => {
        let body = "";
        response.setEncoding("utf8");
        response.on("data", (chunk) => {
          body += chunk;
        });
        response.on("end", () => {
          resolve({ status: response.statusCode, body: JSON.parse(body) });
        });
      }
    );
    request.on("error", reject);
    request.end();
  });
}

test("serves deterministic navigation, redirect, mutation, and form fixtures", async () => {
  const fixture = createFixtureServer();
  const address = await fixture.start();
  try {
    const baseUrl = `${address.protocol}://${address.host}:${address.port}`;
    const health = await fetch(`${baseUrl}/health`);
    assert.equal(health.status, 200);
    assert.deepEqual(await health.json(), {
      fixture_set: "machina-foundation",
      version: "2026-08-09.1",
      origin: "127.0.0.1",
      external_network: false
    });

    const redirect = await fetch(`${baseUrl}/redirect`, { redirect: "manual" });
    assert.equal(redirect.status, 302);
    assert.equal(redirect.headers.get("location"), "/navigation");

    const page = await fetch(`${baseUrl}/navigation`);
    assert.match(await page.text(), /Navigation fixture/);
    assert.equal(page.headers.get("x-machina-fixture-version"), "2026-08-09.1");

    const firstOrigin = await requestOrigin(address, "one.localhost");
    assert.equal(firstOrigin.status, 200);
    assert.deepEqual(firstOrigin.body, {
      origin: "one.localhost",
      fixture_set: "machina-foundation",
      external_network: false
    });
    const secondOrigin = await requestOrigin(address, "two.localhost");
    assert.equal(secondOrigin.status, 200);
    assert.deepEqual(secondOrigin.body, {
      origin: "two.localhost",
      fixture_set: "machina-foundation",
      external_network: false
    });

    const mutation = await fetch(`${baseUrl}/dom-mutation`);
    assert.match(await mutation.text(), /data-state="initial"/);
    const networkPolicy = await fetch(`${baseUrl}/network-policy`);
    assert.deepEqual(await networkPolicy.json(), {
      origin: "127.0.0.1",
      redirect_target: "/navigation",
      private_network_allowed: false
    });

    const form = await fetch(`${baseUrl}/form`, {
      method: "POST",
      headers: { "content-type": "application/x-www-form-urlencoded" },
      body: "name=fixture"
    });
    assert.deepEqual(await form.json(), { accepted: true, body: "name=fixture" });

    const missing = await fetch(`${baseUrl}/missing`);
    assert.equal(missing.status, 404);
    assert.deepEqual(await missing.json(), {
      error: "fixture route not found",
      trace_ref: "fixture/2026-08-09.1/missing"
    });
  } finally {
    await fixture.stop();
  }
});

test("rejects non-loopback fixture binding", () => {
  assert.throws(
    () => createFixtureServer({ host: "0.0.0.0" }),
    /loopback host/
  );
});

test("serves a deterministic WebSocket upgrade handshake", async () => {
  const fixture = createFixtureServer();
  const address = await fixture.start();
  try {
    const response = await new Promise((resolve, reject) => {
      const socket = net.createConnection({
        host: address.host,
        port: address.port
      });
      let received = "";
      socket.setEncoding("utf8");
      socket.on("error", reject);
      socket.on("data", (chunk) => {
        received += chunk;
        if (received.includes("\r\n\r\n")) {
          socket.destroy();
          resolve(received);
        }
      });
      socket.on("connect", () => {
        socket.write(
          [
            "GET /ws HTTP/1.1",
            `Host: ${address.host}:${address.port}`,
            "Upgrade: websocket",
            "Connection: Upgrade",
            "Sec-WebSocket-Version: 13",
            "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==",
            "\r\n"
          ].join("\r\n")
        );
      });
    });
    assert.match(response, /HTTP\/1\.1 101 Switching Protocols/);
    assert.match(
      response,
      /Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK\+xOo=/
    );
  } finally {
    await fixture.stop();
  }
});

test("supports injected short-lived HTTPS fixture certificates", async () => {
  const certificateRoot = await mkdtemp(join(tmpdir(), "machina-fixture-tls-"));
  const keyPath = join(certificateRoot, "key.pem");
  const certificatePath = join(certificateRoot, "cert.pem");
  try {
    const opensslBinary =
      process.env.OPENSSL_BIN ??
      (process.platform === "win32"
        ? join(
            process.env.USERPROFILE ?? "",
            "scoop",
            "apps",
            "openssl",
            "current",
            "bin",
            "openssl.exe"
          )
        : "openssl");
    const openssl = spawnSync(
      existsSync(opensslBinary) ? opensslBinary : "openssl",
      [
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        keyPath,
        "-out",
        certificatePath,
        "-days",
        "1",
        "-subj",
        "/CN=one.localhost"
      ],
      {
        encoding: "utf8",
        shell: false,
        stdio: "pipe",
        env: {
          ...process.env,
          OPENSSL_CONF:
            process.env.OPENSSL_CONF ??
            join(dirname(opensslBinary), "cnf", "openssl.cnf")
        }
      }
    );
    assert.equal(
      openssl.status,
      0,
      openssl.stderr || "openssl certificate generation failed"
    );
    const fixture = createFixtureServer({
      tlsOptions: {
        key: await readFile(keyPath),
        cert: await readFile(certificatePath)
      }
    });
    const address = await fixture.start();
    try {
      const response = await new Promise((resolve, reject) => {
        https
          .get(
            {
              hostname: address.host,
              port: address.port,
              path: "/health",
              rejectUnauthorized: false,
              headers: { host: `one.localhost:${address.port}` }
            },
            (incoming) => {
              let body = "";
              incoming.setEncoding("utf8");
              incoming.on("data", (chunk) => {
                body += chunk;
              });
              incoming.on("end", () =>
                resolve({ status: incoming.statusCode, body: JSON.parse(body) })
              );
            }
          )
          .on("error", reject);
      });
      assert.equal(response.status, 200);
      assert.equal(response.body.fixture_set, "machina-foundation");
    } finally {
      await fixture.stop();
    }
  } finally {
    await rm(certificateRoot, { recursive: true, force: true });
  }
});
