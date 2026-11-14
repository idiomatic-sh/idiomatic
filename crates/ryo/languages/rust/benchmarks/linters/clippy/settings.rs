//! Explicit construction values for the pinned clippy implementation.
use ryo::linter::RuleId;
use std::{collections::BTreeSet, path::PathBuf};

#[derive(Debug)]
/// Pinned executable, exact rule selection and the shared build directory chosen by composition.
pub(crate) struct LinterClippySettings {
    /// The verified `clippy-driver` of the pinned toolchain; the package manager, its clippy command and rustc are its siblings.
    pub executable: PathBuf,
    /// Rules permitted in normalized reference output.
    pub defaults: BTreeSet<RuleId>,
    /// The target directory of every root this adapter runs in, and the configuration directory clippy reads.
    pub targets: PathBuf,
}
