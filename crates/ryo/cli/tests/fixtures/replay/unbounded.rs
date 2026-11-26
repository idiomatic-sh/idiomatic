//! The lint on as many workers as the machine has, which a test runs: a test asks for no worker count. A measurement never does
//! (the lock holds no machine), so the benches state a count of their own.

use ryo_core::inbound::use_cases::lint::types::Rule;
use std::path::Path;

/// What a lint gave: the rows the rules gave, and the files nothing read.
pub(crate) struct Linted {
    /// Each finding as text.
    pub rows: Vec<Vec<String>>,
    /// The files nothing read.
    pub unread: Vec<String>,
}

/// Lints `root` with `rules`, keeping what it found in `cache`.
///
/// # Errors
///
/// What the lint refuses, as text.
pub(crate) fn lint_cached(
    root: &Path,
    rules: Vec<Rule>,
    cache: ryo_sdk::linter::Cache,
) -> Result<Linted, String> {
    let (rows, unread) = super::base::lint_with(root, rules, cache)?;
    Ok(Linted { rows, unread })
}
