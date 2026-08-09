//! Synthetic keyboard event payload. Present now (typed, constructible) but
//! not yet wired into any dispatch call — `interaction.click.v1` does not
//! use it. It exists so a later `interaction.type.v1` task can reuse
//! `crates/events`'s dispatch machinery without a crate redesign.

#[derive(Clone, Debug, PartialEq)]
pub struct KeyboardEventInit {
    pub key: String,
    pub code: String,
    pub repeat: bool,
    pub ctrl_key: bool,
    pub shift_key: bool,
    pub alt_key: bool,
    pub meta_key: bool,
}

impl KeyboardEventInit {
    pub fn new(key: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            code: code.into(),
            repeat: false,
            ctrl_key: false,
            shift_key: false,
            alt_key: false,
            meta_key: false,
        }
    }
}
