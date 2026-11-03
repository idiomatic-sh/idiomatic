//! Public column vocabulary for proven lexical names.
use super::item::QualifiedNameColumn;
/// Query relation name.
#[must_use]
pub fn name() -> &'static str {
    "qualified-name"
}
/// The relation does not claim runtime value identity or implicit target-version availability.
#[must_use]
pub fn summary() -> &'static str {
    "Canonical lexical import/builtin origins of load expressions; dynamic origins remain unknown. Builtin availability is compared numerically by since-major and since-minor against the consumer's language target. Version fields are inapplicable to non-builtin origins."
}
/// Column name exposed to queries.
#[must_use]
pub fn column_name(column: QualifiedNameColumn) -> &'static str {
    match column {
        QualifiedNameColumn::Id => "id",
        QualifiedNameColumn::Path => "path",
        QualifiedNameColumn::Provenance => "provenance",
        QualifiedNameColumn::SinceMajor => "since-major",
        QualifiedNameColumn::SinceMinor => "since-minor",
        QualifiedNameColumn::File => "file",
        QualifiedNameColumn::Span => "span",
    }
}
