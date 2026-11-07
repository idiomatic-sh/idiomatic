// The rows `relations/parameter/input.ts` must give.

use ryo::languages::contract::{
    Expected, ParameterExpectedRow, expected_rows, file, name, node, number, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[ParameterExpectedRow] = &[
    ParameterExpectedRow {
        id: node(7),
        kind: name("positional"),
        refinement: name("identifier"),
        name: name("x"),
        ordinal: number(0),
        owner: node(1),
        file: file("parameter/input.ts"),
        span: span((1, 12), (1, 21)),
    },
    ParameterExpectedRow {
        id: node(16),
        kind: name("variadic-positional"),
        refinement: name("rest-element"),
        name: name("rest"),
        ordinal: number(1),
        owner: node(1),
        file: file("parameter/input.ts"),
        span: span((1, 23), (1, 40)),
    },
    ParameterExpectedRow {
        id: node(53),
        kind: name("positional"),
        refinement: name("identifier"),
        name: name("dto"),
        ordinal: number(0),
        owner: node(46),
        file: file("parameter/input.ts"),
        span: span((3, 34), (3, 42)),
    },
    ParameterExpectedRow {
        id: node(75),
        kind: name("positional"),
        refinement: name("ts-parameter-property"),
        name: name("x"),
        ordinal: number(0),
        owner: node(73),
        file: file("parameter/input.ts"),
        span: span((3, 59), (3, 72)),
    },
    ParameterExpectedRow {
        id: node(100),
        kind: name("positional"),
        refinement: name("identifier"),
        name: name("x"),
        ordinal: number(0),
        owner: node(94),
        file: file("parameter/input.ts"),
        span: span((4, 19), (4, 21)),
    },
    ParameterExpectedRow {
        id: node(105),
        kind: name("positional"),
        refinement: name("identifier"),
        name: name("y"),
        ordinal: number(1),
        owner: node(94),
        file: file("parameter/input.ts"),
        span: span((4, 23), (4, 26)),
    },
    ParameterExpectedRow {
        id: node(121),
        kind: name("receiver"),
        refinement: name("identifier"),
        name: name("this"),
        ordinal: number(0),
        owner: node(115),
        file: file("parameter/input.ts"),
        span: span((5, 16), (5, 28)),
    },
    ParameterExpectedRow {
        id: node(131),
        kind: name("positional"),
        refinement: name("identifier"),
        name: name("x"),
        ordinal: number(1),
        owner: node(115),
        file: file("parameter/input.ts"),
        span: span((5, 30), (5, 39)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
