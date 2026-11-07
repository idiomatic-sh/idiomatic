// The rows `relations/parameter/input.go` must give.

use ryo::languages::contract::{
    Expected, ParameterExpectedRow, expected_rows, file, name, node, number, span, unknown,
};

/// The rows, in the order the language gives them.
const ROWS: &[ParameterExpectedRow] = &[
    ParameterExpectedRow {
        id: node(26),
        kind: name("receiver"),
        refinement: name("receiver"),
        name: name("t"),
        ordinal: number(0),
        owner: node(19),
        file: file("parameter/input.go"),
        span: span((5, 7), (5, 8)),
    },
    ParameterExpectedRow {
        id: node(40),
        kind: name("positional"),
        refinement: name("parameter"),
        name: name("a"),
        ordinal: number(0),
        owner: node(19),
        file: file("parameter/input.go"),
        span: span((5, 17), (5, 18)),
    },
    ParameterExpectedRow {
        id: node(44),
        kind: name("positional"),
        refinement: name("parameter"),
        name: name("b"),
        ordinal: number(1),
        owner: node(19),
        file: file("parameter/input.go"),
        span: span((5, 20), (5, 21)),
    },
    ParameterExpectedRow {
        id: node(52),
        kind: name("variadic-positional"),
        refinement: name("variadic"),
        name: name("rest"),
        ordinal: number(2),
        owner: node(19),
        file: file("parameter/input.go"),
        span: span((5, 27), (5, 31)),
    },
    ParameterExpectedRow {
        id: node(64),
        kind: name("result"),
        refinement: name("result"),
        name: name("n"),
        ordinal: number(0),
        owner: node(19),
        file: file("parameter/input.go"),
        span: span((5, 44), (5, 45)),
    },
    ParameterExpectedRow {
        id: node(72),
        kind: name("result"),
        refinement: name("result"),
        name: name("err"),
        ordinal: number(1),
        owner: node(19),
        file: file("parameter/input.go"),
        span: span((5, 51), (5, 54)),
    },
    ParameterExpectedRow {
        id: node(94),
        kind: name("receiver"),
        refinement: name("receiver"),
        name: unknown("unnamed"),
        ordinal: number(0),
        owner: node(87),
        file: file("parameter/input.go"),
        span: span((7, 7), (7, 8)),
    },
    ParameterExpectedRow {
        id: node(103),
        kind: name("positional"),
        refinement: name("parameter"),
        name: unknown("unnamed"),
        ordinal: number(0),
        owner: node(87),
        file: file("parameter/input.go"),
        span: span((7, 15), (7, 18)),
    },
    ParameterExpectedRow {
        id: node(108),
        kind: name("positional"),
        refinement: name("parameter"),
        name: unknown("unnamed"),
        ordinal: number(1),
        owner: node(87),
        file: file("parameter/input.go"),
        span: span((7, 20), (7, 26)),
    },
    ParameterExpectedRow {
        id: node(146),
        kind: name("positional"),
        refinement: name("parameter"),
        name: name("x"),
        ordinal: number(0),
        owner: node(128),
        file: file("parameter/input.go"),
        span: span((9, 15), (9, 16)),
    },
    ParameterExpectedRow {
        id: node(180),
        kind: name("positional"),
        refinement: name("parameter"),
        name: name("p"),
        ordinal: number(0),
        owner: node(174),
        file: file("parameter/input.go"),
        span: span((10, 12), (10, 13)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
