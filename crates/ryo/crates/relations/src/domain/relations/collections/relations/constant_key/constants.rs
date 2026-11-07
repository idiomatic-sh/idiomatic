//! The vocabulary of the optional `constant-key` relation.

use super::item::ConstantKeyColumn;

/// Query relation name.
#[must_use]
pub fn name() -> &'static str {
    "constant-key"
}

/// Supported parsed constant expressions and their collision-checked canonical representatives.
#[must_use]
pub fn summary() -> &'static str {
    "Optional keys for parser-supported primitive and constant tuple expressions; keys are language-policy facts, not a general evaluator or a claim about unsupported expressions."
}

/// Column name.
#[must_use]
pub fn column_name(column: ConstantKeyColumn) -> &'static str {
    match column {
        ConstantKeyColumn::Id => "id",
        ConstantKeyColumn::Key => "key",
        ConstantKeyColumn::File => "file",
        ConstantKeyColumn::Span => "span",
    }
}
