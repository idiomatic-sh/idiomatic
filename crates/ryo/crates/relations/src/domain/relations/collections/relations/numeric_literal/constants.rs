//! Public vocabulary of the optional parser-decoded numeric relation.

use super::item::NumericLiteralColumn;

/// Query relation name.
#[must_use]
pub fn name() -> &'static str {
    "numeric-literal"
}

/// Relation contract; values describe literal nodes rather than evaluated expressions.
#[must_use]
pub fn summary() -> &'static str {
    "Optional parser-decoded integer or binary64 literal payload, with canonical decimal integer text of arbitrary precision and round-trip float text. Excludes booleans and complex values; unary signs remain syntax. Missing required parser payload makes the relation partial."
}

/// Column spelling.
#[must_use]
pub fn column_name(column: NumericLiteralColumn) -> &'static str {
    match column {
        NumericLiteralColumn::Id => "id",
        NumericLiteralColumn::Kind => "kind",
        NumericLiteralColumn::Value => "value",
        NumericLiteralColumn::File => "file",
        NumericLiteralColumn::Span => "span",
    }
}
