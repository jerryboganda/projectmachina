CREATE TABLE project_credentials (
    credential_id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    token_prefix TEXT NOT NULL,
    token_hash TEXT NOT NULL,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL,
    version BIGINT NOT NULL DEFAULT 1,
    FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, project_id)
);

CREATE TABLE policy_audit (
    audit_id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    credential_id TEXT,
    action TEXT NOT NULL,
    policy_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, project_id)
);

CREATE INDEX project_credentials_scope_idx
    ON project_credentials (organization_id, project_id);
