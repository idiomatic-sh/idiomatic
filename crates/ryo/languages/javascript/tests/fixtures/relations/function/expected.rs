// The rows `relations/function/input.js` must give.

use ryo::languages::contract::{
    Expected, FunctionExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[FunctionExpectedRow] = &[
    FunctionExpectedRow {
        id: node(31),
        name: name("greet"),
        file: file("function/input.js"),
        span: span((3, 8), (6, 2)),
    },
    FunctionExpectedRow {
        id: node(190),
        name: name("#p"),
        file: file("function/input.js"),
        span: span((12, 5), (12, 10)),
    },
    FunctionExpectedRow {
        id: node(201),
        name: name("constructor"),
        file: file("function/input.js"),
        span: span((13, 14), (13, 59)),
    },
    FunctionExpectedRow {
        id: node(259),
        name: name("hello"),
        file: file("function/input.js"),
        span: span((15, 1), (15, 58)),
    },
    FunctionExpectedRow {
        id: node(319),
        name: name("m"),
        file: file("function/input.js"),
        span: span((17, 16), (17, 21)),
    },
    FunctionExpectedRow {
        id: node(349),
        name: name("g"),
        file: file("function/input.js"),
        span: span((17, 47), (17, 63)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
