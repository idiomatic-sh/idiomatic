// The rows `relations/static_call/input.go` must give.

use ryo::languages::contract::{
    Expected, StaticCallExpectedRow, expected_rows, file, name, node, span, unknown,
};

/// The rows, in the order the language gives them.
const ROWS: &[StaticCallExpectedRow] = &[
    StaticCallExpectedRow {
        occurrence: node(39),
        owner: node(13),
        target: name("fmt.Println"),
        file: file("static_call/input.go"),
        span: span((6, 2), (6, 13)),
    },
    StaticCallExpectedRow {
        occurrence: node(52),
        owner: node(13),
        target: unknown("dynamic-callee"),
        file: file("static_call/input.go"),
        span: span((7, 2), (7, 7)),
    },
    StaticCallExpectedRow {
        occurrence: node(64),
        owner: node(13),
        target: unknown("dynamic-callee"),
        file: file("static_call/input.go"),
        span: span((8, 2), (8, 5)),
    },
    StaticCallExpectedRow {
        occurrence: node(65),
        owner: node(13),
        target: name("g"),
        file: file("static_call/input.go"),
        span: span((8, 2), (8, 3)),
    },
    StaticCallExpectedRow {
        occurrence: node(91),
        owner: node(13),
        target: unknown("dynamic-callee"),
        file: file("static_call/input.go"),
        span: span((10, 2), (10, 8)),
    },
    StaticCallExpectedRow {
        occurrence: node(105),
        owner: node(13),
        target: name("a.b.c.d"),
        file: file("static_call/input.go"),
        span: span((11, 2), (11, 9)),
    },
    StaticCallExpectedRow {
        occurrence: node(124),
        owner: node(13),
        target: unknown("dynamic-callee"),
        file: file("static_call/input.go"),
        span: span((12, 2), (12, 11)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
