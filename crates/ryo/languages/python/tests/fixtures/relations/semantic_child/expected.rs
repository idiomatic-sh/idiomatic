// Native projection reuses independently specified native syntax rows.
// The rows `relations/semantic_child/input.py` must give.

use ryo::languages::contract::{
    Expected, SemanticChildExpectedRow, expected_rows, file, name, node, number,
};

/// The rows, in the order the language gives them.
const ROWS: &[SemanticChildExpectedRow] = &[
    SemanticChildExpectedRow {
        parent: node(0),
        child: node(1),
        ordinal: number(0),
        field: name("body"),
        file: file("semantic_child/input.py"),
    },
    SemanticChildExpectedRow {
        parent: node(0),
        child: node(7),
        ordinal: number(1),
        field: name("none"),
        file: file("semantic_child/input.py"),
    },
    SemanticChildExpectedRow {
        parent: node(0),
        child: node(8),
        ordinal: number(2),
        field: name("body"),
        file: file("semantic_child/input.py"),
    },
    SemanticChildExpectedRow {
        parent: node(0),
        child: node(21),
        ordinal: number(3),
        field: name("none"),
        file: file("semantic_child/input.py"),
    },
    SemanticChildExpectedRow {
        parent: node(1),
        child: node(2),
        ordinal: number(0),
        field: name("none"),
        file: file("semantic_child/input.py"),
    },
    SemanticChildExpectedRow {
        parent: node(1),
        child: node(3),
        ordinal: number(1),
        field: name("none"),
        file: file("semantic_child/input.py"),
    },
    SemanticChildExpectedRow {
        parent: node(1),
        child: node(4),
        ordinal: number(2),
        field: name("names"),
        file: file("semantic_child/input.py"),
    },
    SemanticChildExpectedRow {
        parent: node(4),
        child: node(5),
        ordinal: number(0),
        field: name("name"),
        file: file("semantic_child/input.py"),
    },
    SemanticChildExpectedRow {
        parent: node(5),
        child: node(6),
        ordinal: number(0),
        field: name("none"),
        file: file("semantic_child/input.py"),
    },
    SemanticChildExpectedRow {
        parent: node(8),
        child: node(9),
        ordinal: number(0),
        field: name("value"),
        file: file("semantic_child/input.py"),
    },
    SemanticChildExpectedRow {
        parent: node(9),
        child: node(10),
        ordinal: number(0),
        field: name("func"),
        file: file("semantic_child/input.py"),
    },
    SemanticChildExpectedRow {
        parent: node(9),
        child: node(12),
        ordinal: number(1),
        field: name("none"),
        file: file("semantic_child/input.py"),
    },
    SemanticChildExpectedRow {
        parent: node(10),
        child: node(11),
        ordinal: number(0),
        field: name("id"),
        file: file("semantic_child/input.py"),
    },
    SemanticChildExpectedRow {
        parent: node(12),
        child: node(13),
        ordinal: number(0),
        field: name("none"),
        file: file("semantic_child/input.py"),
    },
    SemanticChildExpectedRow {
        parent: node(12),
        child: node(14),
        ordinal: number(1),
        field: name("args"),
        file: file("semantic_child/input.py"),
    },
    SemanticChildExpectedRow {
        parent: node(12),
        child: node(20),
        ordinal: number(2),
        field: name("none"),
        file: file("semantic_child/input.py"),
    },
    SemanticChildExpectedRow {
        parent: node(14),
        child: node(15),
        ordinal: number(0),
        field: name("value"),
        file: file("semantic_child/input.py"),
    },
    SemanticChildExpectedRow {
        parent: node(14),
        child: node(17),
        ordinal: number(1),
        field: name("none"),
        file: file("semantic_child/input.py"),
    },
    SemanticChildExpectedRow {
        parent: node(14),
        child: node(18),
        ordinal: number(2),
        field: name("attr"),
        file: file("semantic_child/input.py"),
    },
    SemanticChildExpectedRow {
        parent: node(15),
        child: node(16),
        ordinal: number(0),
        field: name("id"),
        file: file("semantic_child/input.py"),
    },
    SemanticChildExpectedRow {
        parent: node(18),
        child: node(19),
        ordinal: number(0),
        field: name("none"),
        file: file("semantic_child/input.py"),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
