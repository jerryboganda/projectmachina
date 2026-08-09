DROP TABLE IF EXISTS artifact_access_grants;
ALTER TABLE artifacts
    DROP CONSTRAINT IF EXISTS artifacts_scope_identity;
