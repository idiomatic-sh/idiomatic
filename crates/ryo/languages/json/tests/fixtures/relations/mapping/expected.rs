// The rows `relations/mapping/input.json` must give.

use ryo::languages::contract::{
    Expected, MappingExpectedRow, expected_rows, file, name, node, number,
};

/// The rows, in the order the language gives them.
const ROWS: &[MappingExpectedRow] = &[
    MappingExpectedRow {
        id: node(0),
        kind: name("standard"),
        ordinal: number(0),
        owner: node(0),
        file: file("mapping/input.json"),
    },
    MappingExpectedRow {
        id: node(7),
        kind: name("inline"),
        ordinal: number(0),
        owner: node(4),
        file: file("mapping/input.json"),
    },
    MappingExpectedRow {
        id: node(23),
        kind: name("sequence-member"),
        ordinal: number(0),
        owner: node(23),
        file: file("mapping/input.json"),
    },
    MappingExpectedRow {
        id: node(33),
        kind: name("sequence-member"),
        ordinal: number(1),
        owner: node(33),
        file: file("mapping/input.json"),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
