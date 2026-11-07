//! The rows of a relation written as the fixture writes them: as source text for a fixture, and as a line for a finding.

use super::expected::{Cell, Expected};
use ryo_relations::domain::relations::service::{column_name, columns, folder_of};
use ryo_relations::domain::relations::types::Relation;

/// The name of the function that writes a cell in the fixture's own syntax (`node(1)`, `span((1, 1), (1, 8))`); none for a flag.
fn constructor(cell: &Cell) -> Option<&'static str> {
    match cell {
        Cell::Unlocated => Some("unlocated"),
        Cell::Node(_) => Some("node"),
        Cell::Name(_) => Some("name"),
        Cell::File(_) => Some("file"),
        Cell::Span(..) => Some("span"),
        Cell::Number(_) => Some("number"),
        Cell::Unknown(_) => Some("unknown"),
        Cell::Flag(_) => None,
    }
}

/// The name of the typed row of `relation`.
fn row_type(relation: Relation) -> String {
    format!("{relation:?}ExpectedRow")
}

/// The fields of a row of `relation` written as the fixture writes them, one for each column.
fn fields(relation: Relation, row: &[Cell]) -> Vec<String> {
    columns(relation)
        .into_iter()
        .zip(row)
        .map(|(column, cell)| format!("{}: {cell}", column_name(column)))
        .collect()
}

/// The text of an `expected.rs` for `relation`, whose fixture input is `input`, holding `rows`: what a language that wants the rows it
/// writes to be the rows it expects keeps (`--bless`).
#[must_use]
pub fn expected_source(relation: Relation, input: &str, rows: &[Vec<Cell>]) -> String {
    let folder = folder_of(relation);
    let mut used: Vec<&str> = rows.iter().flatten().filter_map(constructor).collect();
    let row = row_type(relation);
    used.push(&row);
    used.push("expected_rows");
    used.sort_unstable();
    used.dedup();
    let mut text = format!(
        "// The rows `relations/{folder}/{input}` must give.\n\nuse ryo::languages::contract::{{Expected, {}}};\n\n/// The rows, in the order the language gives them.\nconst ROWS: &[{row}] = &[\n",
        used.join(", ")
    );
    for each in rows {
        text.push_str("    ");
        text.push_str(&row);
        text.push_str(" {\n");
        for field in fields(relation, each) {
            text.push_str("        ");
            text.push_str(&field);
            text.push_str(",\n");
        }
        text.push_str("    },\n");
    }
    text.push_str("];\n\n/// The rows the fixture's input must give, as the relations contract compares them.\npub(super) fn expected() -> Expected {\n    expected_rows(ROWS)\n}\n");
    text
}

/// A row of `relation` as a line of text, written as the fixture writes it, for the findings.
#[must_use]
pub fn shown(relation: Relation, row: &[Cell]) -> String {
    format!(
        "{} {{ {} }}",
        row_type(relation),
        fields(relation, row).join(", ")
    )
}
