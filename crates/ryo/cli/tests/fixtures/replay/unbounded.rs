//! The lint on as many workers as the machine has, which a test runs: a test asks for no worker count. A measurement never does
//! (the lock holds no machine), so the benches use `counting::lint_with_workers` with a count of their own.

use super::counting::{Counted, lint_with_workers};
use ryo_core::inbound::use_cases::lint::types::Rule;
use std::path::Path;
use std::sync::Arc;

/// Lints `root` with `rules`, keeping what it found in `lint_store`, and counts the files the language read.
///
/// # Errors
///
/// What the lint refuses, as text.
pub(crate) fn lint_counting(
    root: &Path,
    rules: Vec<Rule>,
    lint_store: Arc<dyn ryo_core::outbound::lint_store::interface::LintStorePort>,
) -> Result<Counted, String> {
    lint_with_workers(root, rules, lint_store, 0)
}
