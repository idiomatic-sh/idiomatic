// The rows `relations/syntax_child/input.json` must give.

use ryo::languages::contract::{
    Expected, SyntaxChildExpectedRow, expected_rows, file, name, node, number,
};

/// The rows, in the order the language gives them.
const ROWS: &[SyntaxChildExpectedRow] = &[
    SyntaxChildExpectedRow {
        parent: node(0),
        child: node(1),
        ordinal: number(0),
        field: name("value"),
        file: file("syntax_child/input.json"),
    },
    SyntaxChildExpectedRow {
        parent: node(0),
        child: node(12),
        ordinal: number(1),
        field: name("none"),
        file: file("syntax_child/input.json"),
    },
    SyntaxChildExpectedRow {
        parent: node(1),
        child: node(2),
        ordinal: number(0),
        field: name("none"),
        file: file("syntax_child/input.json"),
    },
    SyntaxChildExpectedRow {
        parent: node(1),
        child: node(3),
        ordinal: number(1),
        field: name("none"),
        file: file("syntax_child/input.json"),
    },
    SyntaxChildExpectedRow {
        parent: node(1),
        child: node(11),
        ordinal: number(2),
        field: name("none"),
        file: file("syntax_child/input.json"),
    },
    SyntaxChildExpectedRow {
        parent: node(3),
        child: node(4),
        ordinal: number(0),
        field: name("name"),
        file: file("syntax_child/input.json"),
    },
    SyntaxChildExpectedRow {
        parent: node(3),
        child: node(5),
        ordinal: number(1),
        field: name("none"),
        file: file("syntax_child/input.json"),
    },
    SyntaxChildExpectedRow {
        parent: node(3),
        child: node(6),
        ordinal: number(2),
        field: name("none"),
        file: file("syntax_child/input.json"),
    },
    SyntaxChildExpectedRow {
        parent: node(3),
        child: node(7),
        ordinal: number(3),
        field: name("value"),
        file: file("syntax_child/input.json"),
    },
    SyntaxChildExpectedRow {
        parent: node(7),
        child: node(8),
        ordinal: number(0),
        field: name("none"),
        file: file("syntax_child/input.json"),
    },
    SyntaxChildExpectedRow {
        parent: node(7),
        child: node(9),
        ordinal: number(1),
        field: name("value"),
        file: file("syntax_child/input.json"),
    },
    SyntaxChildExpectedRow {
        parent: node(7),
        child: node(10),
        ordinal: number(2),
        field: name("none"),
        file: file("syntax_child/input.json"),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
