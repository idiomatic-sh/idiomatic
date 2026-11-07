// The rows `relations/mapping/input.toml` must give.

use ryo::languages::contract::{Expected, MappingExpectedRow, expected_rows, file, name, node, number};

/// The rows, in the order the language gives them.
const ROWS: &[MappingExpectedRow] = &[
    MappingExpectedRow {
        id: node(0),
        kind: name("standard"),
        ordinal: number(0),
        owner: node(0),
        file: file("mapping/input.toml"),
    },
    MappingExpectedRow {
        id: node(9),
        kind: name("standard"),
        ordinal: number(0),
        owner: node(12),
        file: file("mapping/input.toml"),
    },
    MappingExpectedRow {
        id: node(25),
        kind: name("dotted"),
        ordinal: number(0),
        owner: node(25),
        file: file("mapping/input.toml"),
    },
    MappingExpectedRow {
        id: node(33),
        kind: name("sequence-member"),
        ordinal: number(0),
        owner: node(33),
        file: file("mapping/input.toml"),
    },
    MappingExpectedRow {
        id: node(47),
        kind: name("sequence-member"),
        ordinal: number(1),
        owner: node(47),
        file: file("mapping/input.toml"),
    },
    MappingExpectedRow {
        id: node(61),
        kind: name("standard"),
        ordinal: number(0),
        owner: node(66),
        file: file("mapping/input.toml"),
    },
    MappingExpectedRow {
        id: node(64),
        kind: name("implicit"),
        ordinal: number(0),
        owner: node(64),
        file: file("mapping/input.toml"),
    },
    MappingExpectedRow {
        id: node(75),
        kind: name("inline"),
        ordinal: number(0),
        owner: node(71),
        file: file("mapping/input.toml"),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
