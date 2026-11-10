//! The types of the opt-in lifecycle memory diagnostics: whether they are on, and the phase and owner each one names.

/// Whether the opt-in lifecycle memory diagnostics are on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MemoryDiagnostics {
    /// The diagnostics are written.
    Enabled,
    /// No capacity traversal and no diagnostic span is made.
    Disabled,
}

/// The point of a run a memory diagnostic is written at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MemoryPhase(pub &'static str);

/// What a memory diagnostic measures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MemoryOwner(pub &'static str);
