// The rows `relations/syntax_child/input.css` must give.

use ryo::languages::contract::{
    Expected, SyntaxChildExpectedRow, expected_rows, file, name, node, number,
};

/// The rows, in the order the language gives them.
const ROWS: &[SyntaxChildExpectedRow] = &[
    SyntaxChildExpectedRow {
        parent: node(0),
        child: node(1),
        ordinal: number(0),
        field: name("none"),
        file: file("syntax_child/input.css"),
    },
    SyntaxChildExpectedRow {
        parent: node(0),
        child: node(17),
        ordinal: number(1),
        field: name("none"),
        file: file("syntax_child/input.css"),
    },
    SyntaxChildExpectedRow {
        parent: node(1),
        child: node(2),
        ordinal: number(0),
        field: name("none"),
        file: file("syntax_child/input.css"),
    },
    SyntaxChildExpectedRow {
        parent: node(1),
        child: node(3),
        ordinal: number(1),
        field: name("none"),
        file: file("syntax_child/input.css"),
    },
    SyntaxChildExpectedRow {
        parent: node(1),
        child: node(4),
        ordinal: number(2),
        field: name("none"),
        file: file("syntax_child/input.css"),
    },
    SyntaxChildExpectedRow {
        parent: node(1),
        child: node(8),
        ordinal: number(3),
        field: name("none"),
        file: file("syntax_child/input.css"),
    },
    SyntaxChildExpectedRow {
        parent: node(1),
        child: node(9),
        ordinal: number(4),
        field: name("none"),
        file: file("syntax_child/input.css"),
    },
    SyntaxChildExpectedRow {
        parent: node(1),
        child: node(16),
        ordinal: number(5),
        field: name("none"),
        file: file("syntax_child/input.css"),
    },
    SyntaxChildExpectedRow {
        parent: node(4),
        child: node(5),
        ordinal: number(0),
        field: name("name"),
        file: file("syntax_child/input.css"),
    },
    SyntaxChildExpectedRow {
        parent: node(4),
        child: node(6),
        ordinal: number(1),
        field: name("none"),
        file: file("syntax_child/input.css"),
    },
    SyntaxChildExpectedRow {
        parent: node(4),
        child: node(7),
        ordinal: number(2),
        field: name("value"),
        file: file("syntax_child/input.css"),
    },
    SyntaxChildExpectedRow {
        parent: node(9),
        child: node(10),
        ordinal: number(0),
        field: name("name"),
        file: file("syntax_child/input.css"),
    },
    SyntaxChildExpectedRow {
        parent: node(9),
        child: node(11),
        ordinal: number(1),
        field: name("none"),
        file: file("syntax_child/input.css"),
    },
    SyntaxChildExpectedRow {
        parent: node(9),
        child: node(12),
        ordinal: number(2),
        field: name("value"),
        file: file("syntax_child/input.css"),
    },
    SyntaxChildExpectedRow {
        parent: node(9),
        child: node(13),
        ordinal: number(3),
        field: name("none"),
        file: file("syntax_child/input.css"),
    },
    SyntaxChildExpectedRow {
        parent: node(9),
        child: node(14),
        ordinal: number(4),
        field: name("none"),
        file: file("syntax_child/input.css"),
    },
    SyntaxChildExpectedRow {
        parent: node(9),
        child: node(15),
        ordinal: number(5),
        field: name("important"),
        file: file("syntax_child/input.css"),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
