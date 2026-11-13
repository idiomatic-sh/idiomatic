//! Explicit construction values for the pinned oxlint implementation.
use ryo::linter::RuleId;
use std::{collections::BTreeSet, path::PathBuf};

#[derive(Debug)]
/// Pinned executable and exact rule selection chosen by composition.
pub(crate) struct LinterOxlintSettings {
    /// Explicit reference executable path.
    pub executable: PathBuf,
    /// Rules permitted in normalized reference output.
    pub defaults: BTreeSet<RuleId>,
}
