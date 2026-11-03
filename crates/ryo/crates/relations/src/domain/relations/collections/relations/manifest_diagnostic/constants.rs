//! External names for metadata-validator diagnostics.
use super::item::ManifestDiagnosticColumn;
/// The relation name.
#[must_use]
pub fn name() -> &'static str {
    "manifest-diagnostic"
}
/// The relation meaning.
#[must_use]
pub fn summary() -> &'static str {
    "A source-mapped syntax or schema diagnostic reported by a requested metadata validator; it is not a filesystem read failure."
}
/// A public column name.
#[must_use]
pub fn column_name(column: ManifestDiagnosticColumn) -> &'static str {
    match column {
        ManifestDiagnosticColumn::File => "file",
        ManifestDiagnosticColumn::Span => "span",
        ManifestDiagnosticColumn::LocationKind => "location-kind",
        ManifestDiagnosticColumn::Phase => "phase",
        ManifestDiagnosticColumn::Metadata => "schema",
        ManifestDiagnosticColumn::Code => "code",
        ManifestDiagnosticColumn::Message => "message",
    }
}
