# Evidence — M1-T12 VPS real-Docker runtime verification

## Identity

- Task: M1-T12 follow-up / BLK-003 partial resolution
- Host: owner-controlled shared VPS `185.252.233.186` (domain `machina.polytronx.com`)
- Isolation: dedicated compose project `machina-m1-verify` under
  `/opt/machina-m1-verify` on the VPS; all ports bound to `127.0.0.1` only.
  No nginx-proxy-manager entry, no DNS change, no public exposure. Verified
  no port conflicts with the VPS's other ~20 running containers before start.
- Date: 2026-08-09

## Why this was run

Local development host has no Docker (`Get-Command docker` fails; Docker
Desktop install failed at UAC). `BLK-003` recorded this as blocking the
"external runtime" portion of M1 real-runtime evidence. This run uses the
owner's existing VPS Docker install instead of installing Docker locally.

## Commands and results

```text
$ ssh vps 'for p in 5432 6379 8080 9000 9001 9090; do ss -tln | grep ":$p "; done'
# all six ports free before starting

$ docker compose -f deploy/compose/compose.yaml -p machina-m1-verify up -d
# postgres, redis, object-store (minio), fixture, observability (prometheus)
# all reported "Started"

$ docker compose -f deploy/compose/compose.yaml -p machina-m1-verify ps
NAME                                STATUS
machina-m1-verify-fixture-1         Up 8 seconds (healthy)   127.0.0.1:8080->8080/tcp
machina-m1-verify-object-store-1    Up 8 seconds (healthy)   127.0.0.1:9000-9001->9000-9001/tcp
machina-m1-verify-observability-1   Up 8 seconds (healthy)   127.0.0.1:9090->9090/tcp
machina-m1-verify-postgres-1        Up 8 seconds (healthy)   127.0.0.1:5432->5432/tcp
machina-m1-verify-redis-1           Up 8 seconds (healthy)   127.0.0.1:6379->6379/tcp

$ node --test scripts/test/m1-compatibility-smoke.test.mjs
# ok 1 - runs the injected canonical command-core smoke matrix
# tests 1, pass 1, fail 0

$ docker exec machina-m1-verify-postgres-1 pg_isready -U machina -d machina
/var/run/postgresql:5432 - accepting connections

$ docker exec machina-m1-verify-redis-1 redis-cli ping
PONG

$ curl -s http://127.0.0.1:8080/
{"fixture_set":"machina-foundation","external_network":false}

$ curl -s http://127.0.0.1:9090/-/ready
Prometheus Server is Ready.
```

## What this proves

- Real Docker Engine 29.3.0 can build/run this repository's dev compose
  stack end to end, with real container health checks passing.
- Real Postgres/Redis/MinIO/Prometheus processes respond to genuine
  protocol-level probes (not injected/simulated responses).
- The M1 compatibility smoke test (`scripts/test/m1-compatibility-smoke.mjs`)
  is unaffected by environment — it passes identically on local host and VPS
  because it does not connect to Postgres/Redis/MinIO at all; it uses an
  in-process command core and an in-process fixture HTTP server
  (`scripts/test/fixture-server.mjs`). Running it on the VPS does not add new
  evidence beyond "Node 22 also runs this test on this host."

## What this does NOT prove

- No real Chromium process was launched anywhere in this run.
  `crates/chromium-adapter` has no process-spawn or CDP-connection code
  (verified by search — no `spawn`, `process::Command`, `CDP`, or
  `remote-debugging` references in that crate). The Docker/VPS environment
  gap and the missing-implementation gap are separate; this run only closes
  the former.
- No HTTP/gRPC listener was started or reached from outside the VPS. No
  public network exposure was created; `machina.polytronx.com` still does
  not point at anything from this project.
- This is not a production deployment and makes no availability/SLA claim.

## Cleanup state

Stack left running (healthy, loopback-only) at `/opt/machina-m1-verify` on
the VPS for reuse/inspection. To tear down:

```text
ssh vps 'cd /opt/machina-m1-verify && docker compose -f deploy/compose/compose.yaml -p machina-m1-verify down -v'
```

## Next action

Track real Chromium launch/CDP implementation in `crates/chromium-adapter`
as explicit M3 work (see `BLK-003`), and decide whether this VPS should be a
durable Docker target for CI/dev before relying on it again.
