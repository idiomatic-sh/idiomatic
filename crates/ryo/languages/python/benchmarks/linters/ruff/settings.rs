//! Explicit construction values for the pinned Ruff implementation.
use ryo::linter::RuleId;
use std::{collections::BTreeSet, path::PathBuf};

#[derive(Debug)]
pub(crate) struct LinterRuffSettings {
    pub executable: PathBuf,
    pub defaults: BTreeSet<RuleId>,
}
