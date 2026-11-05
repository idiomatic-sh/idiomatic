// The rows `relations/syntax_error/input.css` must give.

use ryo::languages::contract::{Expected, SyntaxErrorExpectedRow, expected_rows, file, name, node, span};

/// The rows, in the order the language gives them.
const ROWS: &[SyntaxErrorExpectedRow] = &[
    SyntaxErrorExpectedRow {
        id: node(2_147_483_648),
        message: name("a string a line end breaks before its closing quote"),
        phase: name("parse"),
        impact: name("incomplete"),
        code: name("unspecified"),
        file: file("syntax_error/input.css"),
        span: span((2, 15), (2, 17)),
    },
    SyntaxErrorExpectedRow {
        id: node(2_147_483_649),
        message: name("unexpected end of input"),
        phase: name("parse"),
        impact: name("incomplete"),
        code: name("unspecified"),
        file: file("syntax_error/input.css"),
        span: span((4, 1), (5, 1)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
