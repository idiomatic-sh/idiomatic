// The rows `relations/call_argument/input.go` must give.

use ryo::languages::contract::{
    CallArgumentExpectedRow, Expected, expected_rows, file, name, node, number, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[CallArgumentExpectedRow] = &[
    CallArgumentExpectedRow {
        call: node(36),
        position: number(0),
        expression: node(40),
        passes: name("settings"),
        file: file("call_argument/input.go"),
        span: span((4, 10), (4, 18)),
    },
    CallArgumentExpectedRow {
        call: node(36),
        position: number(1),
        expression: node(44),
        passes: name("count"),
        file: file("call_argument/input.go"),
        span: span((4, 20), (4, 26)),
    },
    CallArgumentExpectedRow {
        call: node(36),
        position: number(2),
        expression: node(50),
        passes: name(""),
        file: file("call_argument/input.go"),
        span: span((4, 28), (4, 34)),
    },
    CallArgumentExpectedRow {
        call: node(36),
        position: number(3),
        expression: node(57),
        passes: name("settings"),
        file: file("call_argument/input.go"),
        span: span((4, 36), (4, 47)),
    },
    CallArgumentExpectedRow {
        call: node(67),
        position: number(0),
        expression: node(75),
        passes: name("count"),
        file: file("call_argument/input.go"),
        span: span((5, 9), (5, 14)),
    },
    CallArgumentExpectedRow {
        call: node(67),
        position: number(1),
        expression: node(79),
        passes: name(""),
        file: file("call_argument/input.go"),
        span: span((5, 16), (5, 17)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
