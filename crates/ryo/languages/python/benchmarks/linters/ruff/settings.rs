//! Explicit construction values for the pinned Ruff implementation.
use ryo::linter::RuleId;
use std::{collections::BTreeSet, path::PathBuf};

#[derive(Debug)]
/// Pinned executable and exact rule selection chosen by composition.
pub(crate) struct LinterRuffSettings {
    /// Explicit reference executable path.
    pub executable: PathBuf,
    /// Rules permitted in normalized reference output.
    pub defaults: BTreeSet<RuleId>,
    /// Effective Python version selected by the sealed benchmark profile.
    pub target_version: String,
}
