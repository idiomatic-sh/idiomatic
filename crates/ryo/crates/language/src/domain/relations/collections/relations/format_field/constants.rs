//! The vocabulary of the optional `format-field` relation.
use super::item::FormatFieldColumn;

#[must_use]
pub fn name() -> &'static str {
    "format-field"
}

#[must_use]
pub fn summary() -> &'static str {
    "Optional conversion fields from successfully parsed decoded string formats; each row identifies its positional or mapping argument use."
}

#[must_use]
pub fn column_name(column: FormatFieldColumn) -> &'static str {
    match column {
        FormatFieldColumn::Id => "id",
        FormatFieldColumn::Ordinal => "ordinal",
        FormatFieldColumn::ArgumentKind => "argument-kind",
        FormatFieldColumn::MappingKey => "mapping-key",
        FormatFieldColumn::Conversion => "conversion",
        FormatFieldColumn::WidthMode => "width-mode",
        FormatFieldColumn::PrecisionMode => "precision-mode",
        FormatFieldColumn::File => "file",
        FormatFieldColumn::Span => "span",
    }
}
