//! The types of the `writing` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// Whether a command writes what it makes, or says what it would write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Writing {
    /// It writes.
    Write,
    /// `-n/--dry-run`: it writes nothing.
    DryRun,
}
