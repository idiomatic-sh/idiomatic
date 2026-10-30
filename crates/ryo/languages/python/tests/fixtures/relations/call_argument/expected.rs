// The rows `relations/call_argument/input.py` must give.

use ryo::languages::contract::{
    CallArgumentExpectedRow, Expected, expected_rows, file, name, node, number, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[CallArgumentExpectedRow] = &[
    CallArgumentExpectedRow {
        call: node(23),
        position: number(0),
        expression: node(28),
        passes: name("settings"),
        file: file("call_argument/input.py"),
        span: span((2, 13), (2, 21)),
    },
    CallArgumentExpectedRow {
        call: node(23),
        position: number(1),
        expression: node(32),
        passes: name("count"),
        file: file("call_argument/input.py"),
        span: span((2, 23), (2, 29)),
    },
    CallArgumentExpectedRow {
        call: node(23),
        position: number(2),
        expression: node(38),
        passes: name(""),
        file: file("call_argument/input.py"),
        span: span((2, 31), (2, 37)),
    },
    CallArgumentExpectedRow {
        call: node(23),
        position: number(3),
        expression: node(50),
        passes: name("settings"),
        file: file("call_argument/input.py"),
        span: span((2, 43), (2, 51)),
    },
    CallArgumentExpectedRow {
        call: node(56),
        position: number(0),
        expression: node(65),
        passes: name("count"),
        file: file("call_argument/input.py"),
        span: span((3, 12), (3, 17)),
    },
    CallArgumentExpectedRow {
        call: node(56),
        position: number(1),
        expression: node(69),
        passes: name(""),
        file: file("call_argument/input.py"),
        span: span((3, 19), (3, 20)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
