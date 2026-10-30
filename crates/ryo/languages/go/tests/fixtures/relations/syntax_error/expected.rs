// The rows `relations/syntax_error/input.go` must give.

use ryo::languages::contract::{
    Expected, SyntaxErrorExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[SyntaxErrorExpectedRow] = &[SyntaxErrorExpectedRow {
    id: node(2_147_483_648),
    message: name("syntax error: the string is not terminated"),
    file: file("syntax_error/input.go"),
    span: span((3, 8), (3, 12)),
}];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
