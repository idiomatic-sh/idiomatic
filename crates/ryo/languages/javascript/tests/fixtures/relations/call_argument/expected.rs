// The rows `relations/call_argument/input.js` must give.

use ryo::languages::contract::{
    CallArgumentExpectedRow, Expected, expected_rows, file, name, node, number, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[CallArgumentExpectedRow] = &[
    CallArgumentExpectedRow {
        call: node(19),
        position: number(0),
        expression: node(23),
        passes: name("settings"),
        file: file("call_argument/input.js"),
        span: span((2, 11), (2, 19)),
    },
    CallArgumentExpectedRow {
        call: node(19),
        position: number(1),
        expression: node(27),
        passes: name("count"),
        file: file("call_argument/input.js"),
        span: span((2, 21), (2, 29)),
    },
    CallArgumentExpectedRow {
        call: node(19),
        position: number(2),
        expression: node(33),
        passes: name(""),
        file: file("call_argument/input.js"),
        span: span((2, 31), (2, 37)),
    },
    CallArgumentExpectedRow {
        call: node(19),
        position: number(3),
        expression: node(40),
        passes: name(""),
        file: file("call_argument/input.js"),
        span: span((2, 39), (2, 49)),
    },
    CallArgumentExpectedRow {
        call: node(50),
        position: number(0),
        expression: node(58),
        passes: name("count"),
        file: file("call_argument/input.js"),
        span: span((3, 10), (3, 15)),
    },
    CallArgumentExpectedRow {
        call: node(50),
        position: number(1),
        expression: node(62),
        passes: name(""),
        file: file("call_argument/input.js"),
        span: span((3, 17), (3, 18)),
    },
    CallArgumentExpectedRow {
        call: node(68),
        position: number(0),
        expression: node(74),
        passes: name("settings"),
        file: file("call_argument/input.js"),
        span: span((4, 13), (4, 21)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
