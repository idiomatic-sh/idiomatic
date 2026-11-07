// The rows `relations/static_call/input.js` must give.

use ryo::languages::contract::{
    Expected, StaticCallExpectedRow, expected_rows, file, name, node, span, unknown,
};

/// The rows, in the order the language gives them.
const ROWS: &[StaticCallExpectedRow] = &[
    StaticCallExpectedRow {
        occurrence: node(57),
        owner: node(53),
        target: name("tool.print"),
        file: file("static_call/input.js"),
        span: span((4, 3), (4, 13)),
    },
    StaticCallExpectedRow {
        occurrence: node(79),
        owner: node(53),
        target: name("load"),
        file: file("static_call/input.js"),
        span: span((5, 10), (5, 14)),
    },
    StaticCallExpectedRow {
        occurrence: node(107),
        owner: node(101),
        target: name("greet"),
        file: file("static_call/input.js"),
        span: span((7, 24), (7, 29)),
    },
    StaticCallExpectedRow {
        occurrence: node(117),
        owner: node(0),
        target: name("items.map"),
        file: file("static_call/input.js"),
        span: span((9, 1), (9, 10)),
    },
    StaticCallExpectedRow {
        occurrence: node(209),
        owner: node(205),
        target: name("super"),
        file: file("static_call/input.js"),
        span: span((13, 19), (13, 24)),
    },
    StaticCallExpectedRow {
        occurrence: node(217),
        owner: node(205),
        target: name("this.m"),
        file: file("static_call/input.js"),
        span: span((13, 28), (13, 34)),
    },
    StaticCallExpectedRow {
        occurrence: node(229),
        owner: node(205),
        target: unknown("dynamic-or-anonymous"),
        file: file("static_call/input.js"),
        span: span((13, 38), (13, 43)),
    },
    StaticCallExpectedRow {
        occurrence: node(231),
        owner: node(205),
        target: name("f"),
        file: file("static_call/input.js"),
        span: span((13, 38), (13, 39)),
    },
    StaticCallExpectedRow {
        occurrence: node(244),
        owner: node(205),
        target: name("this.#p"),
        file: file("static_call/input.js"),
        span: span((13, 47), (13, 54)),
    },
    StaticCallExpectedRow {
        occurrence: node(273),
        owner: node(269),
        target: name("hello"),
        file: file("static_call/input.js"),
        span: span((15, 34), (15, 44)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
