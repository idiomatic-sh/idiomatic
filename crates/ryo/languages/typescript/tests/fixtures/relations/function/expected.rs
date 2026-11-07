// The rows `relations/function/input.tsx` must give.

use ryo::languages::contract::{
    Expected, FunctionExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[FunctionExpectedRow] = &[
    FunctionExpectedRow {
        id: node(152),
        name: name("named"),
        file: file("function/input.tsx"),
        span: span((4, 1), (4, 20)),
    },
    FunctionExpectedRow {
        id: node(176),
        name: name("open"),
        file: file("function/input.tsx"),
        span: span((5, 17), (5, 22)),
    },
    FunctionExpectedRow {
        id: node(201),
        name: name("m"),
        file: file("function/input.tsx"),
        span: span((6, 16), (6, 21)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
