// The rows `relations/mapping/input.go.mod` must give.

use ryo::languages::contract::{
    Expected, MappingExpectedRow, expected_rows, file, name, node, number,
};

/// The rows, in the order the language gives them.
const ROWS: &[MappingExpectedRow] = &[
    MappingExpectedRow {
        id: node(0),
        kind: name("implicit"),
        ordinal: number(0),
        owner: node(0),
        file: file("mapping/go.mod"),
    },
    MappingExpectedRow {
        id: node(21),
        kind: name("standard"),
        ordinal: number(0),
        owner: node(0),
        file: file("mapping/go.mod"),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
