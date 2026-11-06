use super::item::FunctionTerminalColumn;
use crate::domain::relations::constants::OUTCOME;
/// The RyoQL relation name.
#[must_use]
pub fn name() -> &'static str {
    "function-terminal"
}
/// A short description for relation listings.
#[must_use]
pub fn summary() -> &'static str {
    "Optional basic terminal behavior of function bodies."
}
/// The RyoQL spelling of a typed column.
#[must_use]
pub fn column_name(column: FunctionTerminalColumn) -> &'static str {
    match column {
        FunctionTerminalColumn::Id => "id",
        FunctionTerminalColumn::Outcome => OUTCOME,
        FunctionTerminalColumn::File => "file",
        FunctionTerminalColumn::Span => "span",
    }
}
