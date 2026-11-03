//! The names of the `source_dependency` relation: the text its name and its columns carry when they leave the engine.

use super::item::SourceDependencyColumn;
use crate::domain::relations::constants::{SOURCE, TARGET};

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "source-dependency"
}

/// What the relation holds, in a sentence.
#[must_use]
pub fn summary() -> &'static str {
    "A path written in one source file that names what another source file owns."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn column_name(column: SourceDependencyColumn) -> &'static str {
    match column {
        SourceDependencyColumn::Source => SOURCE,
        SourceDependencyColumn::Target => TARGET,
        SourceDependencyColumn::Span => "span",
        SourceDependencyColumn::Written => "written",
    }
}
