// Native projection reuses independently specified native syntax rows.
// The rows `relations/semantic_child/input.md` must give.

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
        file: file("semantic_child/input.md"),
    },
    SemanticChildExpectedRow {
        parent: node(1),
        child: node(2),
        ordinal: number(0),
        field: name("children"),
        file: file("semantic_child/input.md"),
    },
    SemanticChildExpectedRow {
        parent: node(1),
        child: node(6),
        ordinal: number(1),
        field: name("none"),
        file: file("semantic_child/input.md"),
    },
    SemanticChildExpectedRow {
        parent: node(1),
        child: node(7),
        ordinal: number(2),
        field: name("children"),
        file: file("semantic_child/input.md"),
    },
    SemanticChildExpectedRow {
        parent: node(2),
        child: node(3),
        ordinal: number(0),
        field: name("none"),
        file: file("semantic_child/input.md"),
    },
    SemanticChildExpectedRow {
        parent: node(2),
        child: node(4),
        ordinal: number(1),
        field: name("children"),
        file: file("semantic_child/input.md"),
    },
    SemanticChildExpectedRow {
        parent: node(2),
        child: node(5),
        ordinal: number(2),
        field: name("none"),
        file: file("semantic_child/input.md"),
    },
    SemanticChildExpectedRow {
        parent: node(7),
        child: node(8),
        ordinal: number(0),
        field: name("children"),
        file: file("semantic_child/input.md"),
    },
    SemanticChildExpectedRow {
        parent: node(7),
        child: node(12),
        ordinal: number(1),
        field: name("children"),
        file: file("semantic_child/input.md"),
    },
    SemanticChildExpectedRow {
        parent: node(8),
        child: node(9),
        ordinal: number(0),
        field: name("none"),
        file: file("semantic_child/input.md"),
    },
    SemanticChildExpectedRow {
        parent: node(8),
        child: node(10),
        ordinal: number(1),
        field: name("children"),
        file: file("semantic_child/input.md"),
    },
    SemanticChildExpectedRow {
        parent: node(8),
        child: node(11),
        ordinal: number(2),
        field: name("none"),
        file: file("semantic_child/input.md"),
    },
    SemanticChildExpectedRow {
        parent: node(12),
        child: node(13),
        ordinal: number(0),
        field: name("none"),
        file: file("semantic_child/input.md"),
    },
    SemanticChildExpectedRow {
        parent: node(12),
        child: node(14),
        ordinal: number(1),
        field: name("children"),
        file: file("semantic_child/input.md"),
    },
    SemanticChildExpectedRow {
        parent: node(12),
        child: node(15),
        ordinal: number(2),
        field: name("none"),
        file: file("semantic_child/input.md"),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
