CREATE TABLE organizations (
    organization_id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    version BIGINT NOT NULL DEFAULT 1
);

CREATE TABLE projects (
    project_id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations (organization_id),
    display_name TEXT NOT NULL,
    policy_version TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    version BIGINT NOT NULL DEFAULT 1,
    UNIQUE (organization_id, project_id)
);

CREATE TABLE policies (
    policy_id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    policy_version TEXT NOT NULL,
    policy_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    version BIGINT NOT NULL DEFAULT 1,
    UNIQUE (project_id, policy_version),
    FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, project_id)
);

CREATE TABLE sessions (
    session_id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    policy_version TEXT NOT NULL,
    state TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    version BIGINT NOT NULL DEFAULT 1,
    UNIQUE (project_id, idempotency_key),
    FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, project_id)
);

CREATE TABLE workers (
    worker_id TEXT PRIMARY KEY,
    organization_id TEXT,
    project_id TEXT,
    engine TEXT NOT NULL,
    capability_snapshot TEXT NOT NULL,
    state TEXT NOT NULL,
    lease_expires_at TIMESTAMPTZ,
    version BIGINT NOT NULL DEFAULT 1,
    CHECK ((organization_id IS NULL) = (project_id IS NULL)),
    FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, project_id)
);

CREATE TABLE artifacts (
    artifact_id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    classification TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    storage_key TEXT NOT NULL,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL,
    version BIGINT NOT NULL DEFAULT 1,
    FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, project_id)
);

CREATE TABLE workflows (
    workflow_id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    name TEXT NOT NULL,
    active_version TEXT,
    created_at TIMESTAMPTZ NOT NULL,
    version BIGINT NOT NULL DEFAULT 1,
    FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, project_id)
);

CREATE TABLE usage_records (
    usage_id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    session_id TEXT,
    metric TEXT NOT NULL,
    quantity NUMERIC NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL,
    FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, project_id)
);

CREATE TABLE audit_records (
    audit_id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations (organization_id),
    project_id TEXT,
    actor_id TEXT NOT NULL,
    action TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    classification TEXT NOT NULL,
    details_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    FOREIGN KEY (organization_id, project_id)
        REFERENCES projects (organization_id, project_id)
);

CREATE INDEX sessions_project_state_idx ON sessions (project_id, state);
CREATE INDEX artifacts_project_expiry_idx ON artifacts (project_id, expires_at);
CREATE INDEX audit_project_created_idx ON audit_records (organization_id, project_id, created_at);
