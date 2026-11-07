// The rows `relations/syntax_tree/input.toml` must give.

use ryo::languages::contract::{Expected, SyntaxTreeExpectedRow, expected_rows, file, name, node, span};

/// The rows, in the order the language gives them.
const ROWS: &[SyntaxTreeExpectedRow] = &[
    SyntaxTreeExpectedRow {
        id: node(0),
        kind: name("toml"),
        text: name(""),
        file: file("syntax_tree/input.toml"),
        span: span((1, 1), (2, 1)),
    },
    SyntaxTreeExpectedRow {
        id: node(1),
        kind: name("keyval"),
        text: name(""),
        file: file("syntax_tree/input.toml"),
        span: span((1, 1), (1, 6)),
    },
    SyntaxTreeExpectedRow {
        id: node(2),
        kind: name("key"),
        text: name(""),
        file: file("syntax_tree/input.toml"),
        span: span((1, 1), (1, 2)),
    },
    SyntaxTreeExpectedRow {
        id: node(3),
        kind: name("unquoted-key"),
        text: name("a"),
        file: file("syntax_tree/input.toml"),
        span: span((1, 1), (1, 2)),
    },
    SyntaxTreeExpectedRow {
        id: node(4),
        kind: name("ws"),
        text: name(" "),
        file: file("syntax_tree/input.toml"),
        span: span((1, 2), (1, 3)),
    },
    SyntaxTreeExpectedRow {
        id: node(5),
        kind: name("keyval-sep"),
        text: name("="),
        file: file("syntax_tree/input.toml"),
        span: span((1, 3), (1, 4)),
    },
    SyntaxTreeExpectedRow {
        id: node(6),
        kind: name("ws"),
        text: name(" "),
        file: file("syntax_tree/input.toml"),
        span: span((1, 4), (1, 5)),
    },
    SyntaxTreeExpectedRow {
        id: node(7),
        kind: name("integer"),
        text: name("1"),
        file: file("syntax_tree/input.toml"),
        span: span((1, 5), (1, 6)),
    },
    SyntaxTreeExpectedRow {
        id: node(8),
        kind: name("newline"),
        text: name("\n"),
        file: file("syntax_tree/input.toml"),
        span: span((1, 6), (2, 1)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
