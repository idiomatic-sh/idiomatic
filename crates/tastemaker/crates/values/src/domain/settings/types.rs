//! The types of the `settings` concept: what identifies the run a request is made in, plain data with no constructor and no inherent
//! impl.

/// What identifies the Tastemaker that runs: a proof, a verdict and a plan are kept under it, so one build never serves another's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Build(pub Box<str>);

/// A process as it starts: its id and when, which make the build of one whose executable cannot be read its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessStart {
    /// The process's id.
    pub id: ProcessId,
    /// When it started.
    pub started: StartedAt,
}

/// A process's id, as the system numbers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessId(pub u32);

/// When a process started, in nanoseconds since the epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StartedAt(pub u128);

/// What reads the bytes of the executable that runs.
pub struct Executable<'a>(pub &'a mut dyn std::io::Read);

impl std::fmt::Debug for Executable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

/// Where a project's store is, and the build that writes it: each use case keeps its own file in the directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Store {
    /// The store's directory.
    pub directory: crate::domain::files::types::MachinePath,
    /// The build that runs.
    pub build: Build,
    /// Whether the store is written, or `-n/--dry-run` keeps it as it was.
    pub writing: crate::domain::writing::types::Writing,
}
