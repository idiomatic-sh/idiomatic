//! Public names for decoded byte-string literals.
use super::item::BytesLiteralColumn;
use crate::domain::relations::constants::OCCURRENCE;
/// Query relation name.
#[must_use]
pub fn name() -> &'static str {
    "bytes-literal"
}
/// Meaning of a row.
#[must_use]
pub fn summary() -> &'static str {
    "Optional parser-decoded byte-string literals; hex-value contains two lowercase hexadecimal digits per octet, with adjacent pieces joined."
}
/// Public column names.
#[must_use]
pub fn column_name(column: BytesLiteralColumn) -> &'static str {
    match column {
        BytesLiteralColumn::Occurrence => OCCURRENCE,
        BytesLiteralColumn::Owner => "owner",
        BytesLiteralColumn::HexValue => "hex-value",
        BytesLiteralColumn::File => "file",
        BytesLiteralColumn::Span => "span",
    }
}
