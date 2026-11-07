// The rows `relations/call_argument/input.rs` must give.

use ryo::languages::contract::{
    CallArgumentExpectedRow, Expected, expected_rows, file, name, node, number, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[CallArgumentExpectedRow] = &[
    CallArgumentExpectedRow {
        call: node(42),
        position: number(0),
        expression: node(49),
        passes: name("settings"),
        file: file("call_argument/input.rs"),
        span: span((1, 50), (1, 58)),
    },
    CallArgumentExpectedRow {
        call: node(42),
        position: number(1),
        expression: node(56),
        passes: name("count"),
        file: file("call_argument/input.rs"),
        span: span((1, 60), (1, 70)),
    },
    CallArgumentExpectedRow {
        call: node(42),
        position: number(2),
        expression: node(67),
        passes: name(""),
        file: file("call_argument/input.rs"),
        span: span((1, 72), (1, 78)),
    },
    CallArgumentExpectedRow {
        call: node(42),
        position: number(3),
        expression: node(78),
        passes: name("settings"),
        file: file("call_argument/input.rs"),
        span: span((1, 80), (1, 89)),
    },
    CallArgumentExpectedRow {
        call: node(96),
        position: number(0),
        expression: node(90),
        passes: name("settings"),
        file: file("call_argument/input.rs"),
        span: span((1, 92), (1, 100)),
    },
    CallArgumentExpectedRow {
        call: node(96),
        position: number(1),
        expression: node(100),
        passes: name("count"),
        file: file("call_argument/input.rs"),
        span: span((1, 104), (1, 109)),
    },
    CallArgumentExpectedRow {
        call: node(96),
        position: number(2),
        expression: node(107),
        passes: name(""),
        file: file("call_argument/input.rs"),
        span: span((1, 111), (1, 112)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
