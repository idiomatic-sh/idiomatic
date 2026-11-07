// The rows `relations/static_call/input.ts` must give.

use ryo::languages::contract::{
    Expected, StaticCallExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[StaticCallExpectedRow] = &[StaticCallExpectedRow {
    occurrence: node(2),
    owner: node(0),
    target: name("foo.bar"),
    file: file("static_call/input.ts"),
    span: span((1, 1), (1, 8)),
}];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
