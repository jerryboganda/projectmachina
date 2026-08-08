//! Canonical Project Machina command model facade.

mod generated;

pub use generated::*;

#[cfg(test)]
mod tests {
    use super::{CommandKind, EngineKind, SCHEMA_VERSION};

    #[test]
    fn exposes_contract_metadata() {
        assert_eq!(SCHEMA_VERSION, "0.1.0");
        assert_ne!(EngineKind::Chromium, EngineKind::Native);
        assert_eq!(CommandKind::SessionCreateV1, CommandKind::SessionCreateV1);
    }
}
