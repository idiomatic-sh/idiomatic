//! Public names and interval semantics for indentation regions.
use super::item::IndentRegionColumn;
use crate::domain::relations::constants::PARENT;
/// Query relation name.
#[must_use]
pub fn name() -> &'static str {
    "indent-region"
}
/// Source intervals are lexical, not AST body spans or inferred visual columns.
#[must_use]
pub fn summary() -> &'static str {
    "Lexical INDENT-to-DEDENT intervals with raw indentation and parent identities; root covers the file, leading trivia before INDENT remains outside, and trailing trivia before DEDENT remains inside."
}
/// Query column name.
#[must_use]
pub fn column_name(column: IndentRegionColumn) -> &'static str {
    match column {
        IndentRegionColumn::Id => "id",
        IndentRegionColumn::Parent => PARENT,
        IndentRegionColumn::Indentation => "indentation",
        IndentRegionColumn::StartByte => "start-byte",
        IndentRegionColumn::EndByte => "end-byte",
        IndentRegionColumn::File => "file",
        IndentRegionColumn::Span => "span",
    }
}
