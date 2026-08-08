bootstrap:
    node scripts/build/bootstrap.mjs

doctor:
    node scripts/build/doctor.mjs

doctor-strict:
    node scripts/build/doctor.mjs --strict

build:
    node scripts/build/run.mjs build

check:
    node scripts/build/run.mjs check

fmt-check:
    node scripts/build/run.mjs fmt-check

test:
    node scripts/build/run.mjs test

contract-check:
    node scripts/contracts/check.mjs
    node scripts/contracts/roundtrip.mjs

security-fast:
    node scripts/security/check.mjs
    node scripts/security/check-supply-chain.mjs

smoke:
    node --test scripts/test/fixture-server.test.mjs benchmarks/harness/runner.test.mjs

check-changed:
    node scripts/build/run.mjs check

test-changed:
    node scripts/build/run.mjs test

clean:
    node scripts/build/clean.mjs

dev-up:
    node scripts/dev/local.mjs up

dev-down:
    node scripts/dev/local.mjs down

dev-health:
    node scripts/dev/local.mjs health

dev-reset:
    node scripts/dev/local.mjs reset --confirm
