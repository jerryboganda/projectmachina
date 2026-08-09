# Artifact and reproduction bundle boundary

M1-T10 provides the storage contract used by traces, failure diagnostics, and
classified reproduction bundles:

- `ArtifactStore` accepts caller-supplied ciphertext plus an encryption-key
  reference; it never decrypts, logs, or exposes the object without an exact
  organization/project scope match.
- Metadata includes purpose, classification, checksum, byte length, storage
  key, creation/expiry, and key reference.
- `SignedArtifactUrl` is scoped to the owning project, HMAC-signed, and capped
  by both the requested TTL and artifact expiry.
- `ReproductionBundle` contains redacted trace events and artifact metadata
  (never object bytes), validates event/hash/checksum integrity, and rejects
  seeded canary/sensitive metadata.

The PostgreSQL `artifact_access_grants` migration is the durable grant/audit
projection. Production object storage must perform encryption at rest and
transactionally record issuance and revocation; this crate is the deterministic
contract and focused test implementation.
