//! Effective project policy and stable hash primitives.

use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectivePolicy {
    pub fallback_mode: String,
    pub isolation_tier: String,
    pub egress_mode: String,
    pub retention_days: u32,
    pub approval_mode: String,
}

impl EffectivePolicy {
    pub fn validate(&self) -> Result<(), PolicyError> {
        if self.fallback_mode.trim().is_empty()
            || self.isolation_tier.trim().is_empty()
            || self.egress_mode.trim().is_empty()
            || self.approval_mode.trim().is_empty()
            || self.retention_days == 0
        {
            return Err(PolicyError::InvalidPolicy);
        }
        Ok(())
    }

    pub fn stable_hash(&self) -> Result<String, PolicyError> {
        self.validate()?;
        let canonical = format!(
            "approval_mode={}\negress_mode={}\nfallback_mode={}\nisolation_tier={}\nretention_days={}\n",
            self.approval_mode,
            self.egress_mode,
            self.fallback_mode,
            self.isolation_tier,
            self.retention_days
        );
        let digest = Sha256::digest(canonical.as_bytes());
        Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolicyError {
    InvalidPolicy,
}

#[cfg(test)]
mod tests {
    use super::{EffectivePolicy, PolicyError};

    fn policy() -> EffectivePolicy {
        EffectivePolicy {
            fallback_mode: "prefer-native".to_owned(),
            isolation_tier: "dedicated-process".to_owned(),
            egress_mode: "allowlisted".to_owned(),
            retention_days: 7,
            approval_mode: "high-impact".to_owned(),
        }
    }

    #[test]
    fn policy_hash_is_stable_for_equal_effective_values() {
        assert_eq!(policy().stable_hash(), policy().stable_hash());
        assert_eq!(policy().stable_hash().expect("hash").len(), 64);
    }

    #[test]
    fn invalid_policy_is_rejected() {
        let invalid = EffectivePolicy {
            retention_days: 0,
            ..policy()
        };
        assert_eq!(invalid.stable_hash(), Err(PolicyError::InvalidPolicy));
    }
}
