//! The names of the `render` concept: the headings of the answers it renders.

/// The heading of a finding's rule.
#[must_use]
pub(crate) const fn rule_heading() -> &'static str {
    "rule"
}

/// The heading of a finding's file.
#[must_use]
pub(crate) const fn path_heading() -> &'static str {
    "path"
}

/// The heading of the part of a finding's row that identifies it: how many key values, then each.
#[must_use]
pub(crate) const fn key_heading() -> &'static str {
    "key"
}

/// The heading of the part of a finding's row that holds its columns: each column's heading, then its value.
#[must_use]
pub(crate) const fn columns_heading() -> &'static str {
    "columns"
}

/// The source line of a finding.
#[must_use]
pub(crate) const fn line_heading() -> &'static str {
    "line"
}

/// The source column of a finding.
#[must_use]
pub(crate) const fn column_heading() -> &'static str {
    "column"
}
