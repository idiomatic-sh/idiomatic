//! Public vocabulary of standard format-specification observations.
use super::item::FormatSpecificationColumn;
use crate::domain::relations::constants::{CONVERSION, ORDINAL, ORIGIN, OUTCOME};
/// Public relation name.
#[must_use]
pub fn name() -> &'static str {
    "format-specification"
}
/// Row meaning.
#[must_use]
pub fn summary() -> &'static str {
    "Optional decoded brace-string and f-string format specifications, field ancestry, source origins and target-version standard mini-language outcomes; custom __format__ semantics and call policy are separate."
}
/// Column names.
#[must_use]
pub fn column_name(column: FormatSpecificationColumn) -> &'static str {
    match column {
        FormatSpecificationColumn::Id => "id",
        FormatSpecificationColumn::Ordinal => ORDINAL,
        FormatSpecificationColumn::ParentOrdinal => "parent-ordinal",
        FormatSpecificationColumn::Origin => ORIGIN,
        FormatSpecificationColumn::SourceKind => "source-kind",
        FormatSpecificationColumn::Value => "value",
        FormatSpecificationColumn::Outcome => OUTCOME,
        FormatSpecificationColumn::InvalidType => "invalid-type",
        FormatSpecificationColumn::Selector => "selector",
        FormatSpecificationColumn::Conversion => CONVERSION,
        FormatSpecificationColumn::File => "file",
        FormatSpecificationColumn::Span => "span",
    }
}
