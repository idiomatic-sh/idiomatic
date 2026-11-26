// The rows `relations/scope/input.html` must give.

use ryo::languages::contract::{Expected, ScopeExpectedRow, expected_rows, file, node, span};

/// The rows, in the order the language gives them.
const ROWS: &[ScopeExpectedRow] = &[
    ScopeExpectedRow {
        parent: node(0),
        child: node(0),
        file: file("scope/input.html"),
        span: span((1, 1), (2, 1)),
    },
    ScopeExpectedRow {
        parent: node(0),
        child: node(21),
        file: file("scope/input.html"),
        span: span((1, 48), (1, 196)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
