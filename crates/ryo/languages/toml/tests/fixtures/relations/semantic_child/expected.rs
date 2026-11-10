// Native projection reuses independently specified native syntax rows.
// The rows `relations/semantic_child/input.toml` must give.

use ryo::languages::contract::{
    Expected, SemanticChildExpectedRow, expected_rows, file, name, node, number,
};

/// The rows, in the order the language gives them.
const ROWS: &[SemanticChildExpectedRow] = &[
    SemanticChildExpectedRow {
        parent: node(0),
        child: node(1),
        ordinal: number(0),
        field: name("none"),
        file: file("semantic_child/input.toml"),
    },
    SemanticChildExpectedRow {
        parent: node(0),
        child: node(8),
        ordinal: number(1),
        field: name("none"),
        file: file("semantic_child/input.toml"),
    },
    SemanticChildExpectedRow {
        parent: node(1),
        child: node(2),
        ordinal: number(0),
        field: name("key"),
        file: file("semantic_child/input.toml"),
    },
    SemanticChildExpectedRow {
        parent: node(1),
        child: node(4),
        ordinal: number(1),
        field: name("none"),
        file: file("semantic_child/input.toml"),
    },
    SemanticChildExpectedRow {
        parent: node(1),
        child: node(5),
        ordinal: number(2),
        field: name("none"),
        file: file("semantic_child/input.toml"),
    },
    SemanticChildExpectedRow {
        parent: node(1),
        child: node(6),
        ordinal: number(3),
        field: name("none"),
        file: file("semantic_child/input.toml"),
    },
    SemanticChildExpectedRow {
        parent: node(1),
        child: node(7),
        ordinal: number(4),
        field: name("val"),
        file: file("semantic_child/input.toml"),
    },
    SemanticChildExpectedRow {
        parent: node(2),
        child: node(3),
        ordinal: number(0),
        field: name("simple-key"),
        file: file("semantic_child/input.toml"),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
