//! Synthetic mouse event payload. Present for `mousedown`/`mouseup`/`click`
//! (the `interaction.click.v1` surface); real pointer geometry needs M3-T12
//! (layout/hit-testing) and is out of scope here — see
//! [`MouseEventInit::synthetic_click`]'s doc comment.

/// Which mouse button an event reports, mirroring the DOM `MouseEvent`
/// button enumeration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MouseButton {
    Main,
    Auxiliary,
    Secondary,
    Fourth,
    Fifth,
}

impl MouseButton {
    /// The DOM `MouseEvent.button` numeric code.
    pub fn code(self) -> i16 {
        match self {
            Self::Main => 0,
            Self::Auxiliary => 1,
            Self::Secondary => 2,
            Self::Fourth => 3,
            Self::Fifth => 4,
        }
    }
}

/// Construction parameters for a synthetic mouse event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MouseEventInit {
    pub button: MouseButton,
    pub buttons: u16,
    pub detail: i32,
    pub client_x: f64,
    pub client_y: f64,
    pub ctrl_key: bool,
    pub shift_key: bool,
    pub alt_key: bool,
    pub meta_key: bool,
}

impl MouseEventInit {
    /// Defaults used by `perform_click`: main button, single click,
    /// coordinates `(0.0, 0.0)`, no modifier keys. `ClickPayload` (the
    /// `interaction.click.v1` schema) only carries a selector, so this crate
    /// synthesizes the rest — deliberate and explicitly scoped, not an
    /// oversight; real click coordinates need M3-T12 layout geometry, which
    /// does not exist yet.
    pub fn synthetic_click() -> Self {
        Self {
            button: MouseButton::Main,
            buttons: 1,
            detail: 1,
            client_x: 0.0,
            client_y: 0.0,
            ctrl_key: false,
            shift_key: false,
            alt_key: false,
            meta_key: false,
        }
    }
}
