// The rows `relations/function/input.go` must give.

use ryo::languages::contract::{
    Expected, FunctionExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[FunctionExpectedRow] = &[
    FunctionExpectedRow {
        id: node(19),
        name: name("f"),
        file: file("function/input.go"),
        span: span((5, 1), (5, 12)),
    },
    FunctionExpectedRow {
        id: node(33),
        name: name("set"),
        file: file("function/input.go"),
        span: span((7, 1), (7, 33)),
    },
    FunctionExpectedRow {
        id: node(75),
        name: name("g"),
        file: file("function/input.go"),
        span: span((9, 1), (9, 34)),
    },
    FunctionExpectedRow {
        id: node(116),
        name: name("external"),
        file: file("function/input.go"),
        span: span((11, 1), (11, 25)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
