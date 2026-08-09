//! Authentication and tenant authorization primitives.
//!
//! Plaintext credentials are accepted only at creation/use boundaries and are
//! never retained in the store. Resource access requires an exact organization
//! and project scope match.

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ProjectScope {
    pub organization_id: String,
    pub project_id: String,
}

impl ProjectScope {
    pub fn new(
        organization_id: impl Into<String>,
        project_id: impl Into<String>,
    ) -> Result<Self, AuthError> {
        let organization_id = organization_id.into();
        let project_id = project_id.into();
        if organization_id.trim().is_empty() || project_id.trim().is_empty() {
            return Err(AuthError::InvalidScope);
        }
        Ok(Self {
            organization_id,
            project_id,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizationContext {
    pub actor_id: String,
    pub scope: ProjectScope,
    pub policy_hash: String,
}

impl AuthorizationContext {
    pub fn can_access(&self, requested: &ProjectScope) -> bool {
        &self.scope == requested
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CredentialMetadata {
    pub credential_id: String,
    pub scope: ProjectScope,
    pub token_prefix: String,
    pub token_hash: String,
    pub revoked: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthError {
    InvalidScope,
    InvalidCredential,
    CredentialNotFound,
    CredentialRevoked,
    InvalidToken,
    AccessDenied,
}

impl Display for AuthError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidScope => "organization and project scope are required",
            Self::InvalidCredential => "credential fields are invalid",
            Self::CredentialNotFound => "credential not found",
            Self::CredentialRevoked => "credential revoked",
            Self::InvalidToken => "credential token is invalid",
            Self::AccessDenied => "resource access denied",
        })
    }
}

impl std::error::Error for AuthError {}

#[derive(Clone, Debug, Default)]
pub struct CredentialStore {
    credentials: BTreeMap<String, CredentialMetadata>,
}

impl CredentialStore {
    pub fn create(
        &mut self,
        credential_id: impl Into<String>,
        scope: ProjectScope,
        token: &str,
    ) -> Result<CredentialMetadata, AuthError> {
        let credential_id = credential_id.into();
        if credential_id.trim().is_empty() || token.len() < 16 {
            return Err(AuthError::InvalidCredential);
        }
        if self.credentials.contains_key(&credential_id) {
            return Err(AuthError::InvalidCredential);
        }
        let metadata = CredentialMetadata {
            credential_id: credential_id.clone(),
            scope,
            token_prefix: token.chars().take(8).collect(),
            token_hash: hash_token(token),
            revoked: false,
        };
        self.credentials.insert(credential_id, metadata.clone());
        Ok(metadata)
    }

    pub fn authenticate(
        &self,
        credential_id: &str,
        token: &str,
        requested_scope: &ProjectScope,
        policy_hash: impl Into<String>,
    ) -> Result<AuthorizationContext, AuthError> {
        let credential = self
            .credentials
            .get(credential_id)
            .ok_or(AuthError::CredentialNotFound)?;
        if credential.revoked {
            return Err(AuthError::CredentialRevoked);
        }
        if !constant_time_equal(&credential.token_hash, &hash_token(token)) {
            return Err(AuthError::InvalidToken);
        }
        if !credential.scope.eq(requested_scope) {
            return Err(AuthError::AccessDenied);
        }
        Ok(AuthorizationContext {
            actor_id: credential.credential_id.clone(),
            scope: credential.scope.clone(),
            policy_hash: policy_hash.into(),
        })
    }

    pub fn revoke(&mut self, credential_id: &str) -> Result<(), AuthError> {
        let credential = self
            .credentials
            .get_mut(credential_id)
            .ok_or(AuthError::CredentialNotFound)?;
        credential.revoked = true;
        Ok(())
    }
}

pub fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn constant_time_equal(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.bytes()
        .zip(right.bytes())
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

#[cfg(test)]
mod tests {
    use super::{AuthError, CredentialStore, ProjectScope};

    #[test]
    fn authenticates_scoped_credentials_and_rejects_cross_tenant_access() {
        let mut store = CredentialStore::default();
        let scope = ProjectScope::new("org-1", "project-1").expect("scope");
        let other = ProjectScope::new("org-2", "project-2").expect("scope");
        let metadata = store
            .create("cred-1", scope.clone(), "machina-secret-token")
            .expect("credential");
        assert!(!metadata.token_hash.is_empty());
        let context = store
            .authenticate("cred-1", "machina-secret-token", &scope, "policy-hash")
            .expect("authenticate");
        assert!(context.can_access(&scope));
        assert_eq!(
            store.authenticate("cred-1", "machina-secret-token", &other, "policy-hash"),
            Err(AuthError::AccessDenied)
        );
    }

    #[test]
    fn revocation_and_invalid_tokens_fail_closed() {
        let mut store = CredentialStore::default();
        let scope = ProjectScope::new("org-1", "project-1").expect("scope");
        store
            .create("cred-1", scope.clone(), "machina-secret-token")
            .expect("credential");
        assert_eq!(
            store.authenticate("cred-1", "wrong-secret-token", &scope, "policy"),
            Err(AuthError::InvalidToken)
        );
        store.revoke("cred-1").expect("revoke");
        assert_eq!(
            store.authenticate("cred-1", "machina-secret-token", &scope, "policy"),
            Err(AuthError::CredentialRevoked)
        );
    }
}
