import assert from "node:assert/strict";
import net from "node:net";
import test from "node:test";
import { createFixtureServer } from "./fixture-server.mjs";

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

    const form = await fetch(`${baseUrl}/form`, {
      method: "POST",
      headers: { "content-type": "application/x-www-form-urlencoded" },
      body: "name=fixture"
    });
    assert.deepEqual(await form.json(), { accepted: true, body: "name=fixture" });
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
    assert.match(response, /Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK\+xOo=/);
  } finally {
    await fixture.stop();
  }
});
