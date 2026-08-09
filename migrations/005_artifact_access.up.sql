ALTER TABLE artifacts
    ADD CONSTRAINT artifacts_scope_identity
    UNIQUE (artifact_id, organization_id, project_id);

CREATE TABLE artifact_access_grants (
    grant_id TEXT PRIMARY KEY,
    artifact_id TEXT NOT NULL,
    organization_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    signature_hash TEXT NOT NULL,
    issued_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, project_id),
    FOREIGN KEY (artifact_id, organization_id, project_id)
        REFERENCES artifacts (artifact_id, organization_id, project_id)
);

CREATE INDEX artifact_access_grants_expiry_idx
    ON artifact_access_grants (organization_id, project_id, expires_at)
    WHERE revoked_at IS NULL;
