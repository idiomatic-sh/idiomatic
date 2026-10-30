// The rows `relations/call_argument/input.ts` must give.

use ryo::languages::contract::{
    CallArgumentExpectedRow, Expected, expected_rows, file, name, node, number, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[CallArgumentExpectedRow] = &[
    CallArgumentExpectedRow {
        call: node(30),
        position: number(0),
        expression: node(40),
        passes: name("settings"),
        file: file("call_argument/input.ts"),
        span: span((2, 14), (2, 22)),
    },
    CallArgumentExpectedRow {
        call: node(30),
        position: number(1),
        expression: node(44),
        passes: name("count"),
        file: file("call_argument/input.ts"),
        span: span((2, 24), (2, 32)),
    },
    CallArgumentExpectedRow {
        call: node(30),
        position: number(2),
        expression: node(50),
        passes: name(""),
        file: file("call_argument/input.ts"),
        span: span((2, 34), (2, 40)),
    },
    CallArgumentExpectedRow {
        call: node(30),
        position: number(3),
        expression: node(57),
        passes: name(""),
        file: file("call_argument/input.ts"),
        span: span((2, 42), (2, 52)),
    },
    CallArgumentExpectedRow {
        call: node(67),
        position: number(0),
        expression: node(75),
        passes: name("count"),
        file: file("call_argument/input.ts"),
        span: span((3, 10), (3, 15)),
    },
    CallArgumentExpectedRow {
        call: node(67),
        position: number(1),
        expression: node(79),
        passes: name(""),
        file: file("call_argument/input.ts"),
        span: span((3, 17), (3, 18)),
    },
    CallArgumentExpectedRow {
        call: node(85),
        position: number(0),
        expression: node(91),
        passes: name("settings"),
        file: file("call_argument/input.ts"),
        span: span((4, 13), (4, 21)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
