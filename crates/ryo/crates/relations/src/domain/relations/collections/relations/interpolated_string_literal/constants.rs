//! The vocabulary of the optional `interpolated-string-literal` relation.

use super::item::InterpolatedStringLiteralColumn;

/// Query relation name.
#[must_use]
pub fn name() -> &'static str {
    "interpolated-string-literal"
}

/// Parser-decoded literal fragments, scoped to their owning interpolated string.
#[must_use]
pub fn summary() -> &'static str {
    "Optional parser-decoded literal fragments inside interpolated strings, with their owner; interpolation expressions and format specifications are not fragments."
}

/// Column name.
#[must_use]
pub fn column_name(column: InterpolatedStringLiteralColumn) -> &'static str {
    match column {
        InterpolatedStringLiteralColumn::Id => "id",
        InterpolatedStringLiteralColumn::Owner => "owner",
        InterpolatedStringLiteralColumn::Value => "value",
        InterpolatedStringLiteralColumn::File => "file",
        InterpolatedStringLiteralColumn::Span => "span",
    }
}
