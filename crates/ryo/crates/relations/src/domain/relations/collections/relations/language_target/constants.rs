//! The vocabulary of `language-target`.
use super::item::LanguageTargetColumn;

/// Return the canonical relation name.
#[must_use]
pub fn name() -> &'static str {
    "language-target"
}

/// Describe the data represented by the relation.
#[must_use]
pub fn summary() -> &'static str {
    "The parser language target configured for a source file, as major and minor version numbers."
}

/// Return the canonical name of a relation column.
#[must_use]
pub fn column_name(column: LanguageTargetColumn) -> &'static str {
    match column {
        LanguageTargetColumn::File => "file",
        LanguageTargetColumn::Major => "major",
        LanguageTargetColumn::Minor => "minor",
    }
}
