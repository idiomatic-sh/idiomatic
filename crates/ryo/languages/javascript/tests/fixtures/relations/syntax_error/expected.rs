// The rows `relations/syntax_error/input.js` must give.

use ryo::languages::contract::{
    Expected, SyntaxErrorExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[SyntaxErrorExpectedRow] = &[SyntaxErrorExpectedRow {
    id: node(2_147_483_648),
    message: name("Identifier `duplicate` has already been declared"),
    file: file("syntax_error/input.js"),
    span: span((1, 5), (1, 14)),
}];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
