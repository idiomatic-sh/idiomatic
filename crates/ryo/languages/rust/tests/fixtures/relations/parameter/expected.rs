// The rows `relations/parameter/input.rs` must give.

use ryo::languages::contract::{
    Expected, ParameterExpectedRow, expected_rows, file, name, node, number, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[ParameterExpectedRow] = &[
    ParameterExpectedRow {
        id: node(28),
        kind: name("positional"),
        refinement: name("positional"),
        name: name("left"),
        ordinal: number(0),
        owner: node(23),
        file: file("parameter/input.rs"),
        span: span((1, 23), (1, 27)),
    },
    ParameterExpectedRow {
        id: node(34),
        kind: name("positional"),
        refinement: name("positional"),
        name: name("right"),
        ordinal: number(1),
        owner: node(23),
        file: file("parameter/input.rs"),
        span: span((1, 29), (1, 38)),
    },
    ParameterExpectedRow {
        id: node(73),
        kind: name("receiver"),
        refinement: name("receiver"),
        name: name("self"),
        ordinal: number(0),
        owner: node(66),
        file: file("parameter/input.rs"),
        span: span((3, 17), (3, 22)),
    },
    ParameterExpectedRow {
        id: node(81),
        kind: name("positional"),
        refinement: name("positional"),
        name: name("value"),
        ordinal: number(1),
        owner: node(66),
        file: file("parameter/input.rs"),
        span: span((3, 24), (3, 33)),
    },
    ParameterExpectedRow {
        id: node(92),
        kind: name("positional"),
        refinement: name("positional"),
        name: name(""),
        ordinal: number(2),
        owner: node(66),
        file: file("parameter/input.rs"),
        span: span((3, 35), (3, 51)),
    },
    ParameterExpectedRow {
        id: node(139),
        kind: name("positional"),
        refinement: name("positional"),
        name: name("value"),
        ordinal: number(0),
        owner: node(130),
        file: file("parameter/input.rs"),
        span: span((5, 8), (5, 17)),
    },
    ParameterExpectedRow {
        id: node(175),
        kind: name("positional"),
        refinement: name("positional"),
        name: name("format"),
        ordinal: number(0),
        owner: node(166),
        file: file("parameter/input.rs"),
        span: span((7, 28), (7, 38)),
    },
    ParameterExpectedRow {
        id: node(186),
        kind: name("variadic-positional"),
        refinement: name("variadic"),
        name: name(""),
        ordinal: number(1),
        owner: node(166),
        file: file("parameter/input.rs"),
        span: span((7, 40), (7, 43)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
