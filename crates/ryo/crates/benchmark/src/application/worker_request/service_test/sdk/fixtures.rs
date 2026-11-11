//! The rule files the `sdk` group's scenarios read.

use std::path::Path;

/// Writes the query of rule `id` beneath `rules`, as the SDK reads a rule directory.
pub(super) fn write_rule(rules: &Path, id: &str, query: &str) {
    std::fs::create_dir_all(rules.join(id)).unwrap();
    std::fs::write(rules.join(id).join("query.ryoql"), query).unwrap();
}
