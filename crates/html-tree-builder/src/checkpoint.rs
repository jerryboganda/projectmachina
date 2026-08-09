//! Parser-blocking script checkpoints (design §6) — the pause/resume
//! contract `M2-T09` will drive to actually run a script before parsing
//! continues. `TreeBuilder` never owns `Document`; every driving method
//! takes `&mut Document` per call, so pausing is just "return from
//! `feed`/`finish`" with no self-referential struct.

use machina_dom::ElementHandle;

/// Where a checkpointed `<script>` element's body will need to come from —
/// tracked, not executed, by this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptSource {
    /// `<script>...inline...</script>` with no `src` attribute observed at
    /// the time the checkpoint fired.
    Inline,
    /// `<script src="...">`.
    External,
}

/// A parser-blocking script checkpoint: the `<script>` element and its
/// child text node are fully constructed in the DOM (matching spec
/// ordering) before this is returned. T04's job is to *track* checkpoints,
/// not execute them — every HTML-namespace `</script>` unconditionally
/// produces one, deliberately conservative (no async/defer/module
/// classification; that is a future scripting-engine decision, design §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptCheckpoint {
    pub script_element: ElementHandle,
    pub source: ScriptSource,
}

/// What one `feed`/`finish`/`resume_after_script` call produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeBuilderOutcome {
    /// The tokenizer ran out of buffered input; call `feed` again (or
    /// `finish` to declare end-of-stream).
    NeedsMoreInput,
    /// Parsing is paused at a script checkpoint; the caller must call
    /// `resume_after_script` before driving the builder further.
    ScriptCheckpoint(ScriptCheckpoint),
    /// `Token::Eof` was processed and tree construction is complete.
    Done,
}
