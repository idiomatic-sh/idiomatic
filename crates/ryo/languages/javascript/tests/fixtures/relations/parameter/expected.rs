// The rows `relations/parameter/input.js` must give.

use ryo::languages::contract::{
    Expected, ParameterExpectedRow, expected_rows, file, name, node, number, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[ParameterExpectedRow] = &[
    ParameterExpectedRow {
        id: node(37),
        kind: name("positional"),
        refinement: name("assignment-pattern"),
        name: name("name"),
        ordinal: number(0),
        owner: node(31),
        file: file("parameter/input.js"),
        span: span((3, 23), (3, 37)),
    },
    ParameterExpectedRow {
        id: node(47),
        kind: name("variadic-positional"),
        refinement: name("rest-element"),
        name: name("rest"),
        ordinal: number(1),
        owner: node(31),
        file: file("parameter/input.js"),
        span: span((3, 39), (3, 46)),
    },
    ParameterExpectedRow {
        id: node(102),
        kind: name("positional"),
        refinement: name("identifier"),
        name: name("value"),
        ordinal: number(0),
        owner: node(101),
        file: file("parameter/input.js"),
        span: span((7, 15), (7, 20)),
    },
    ParameterExpectedRow {
        id: node(126),
        kind: name("positional"),
        refinement: name("identifier"),
        name: name("item"),
        ordinal: number(0),
        owner: node(125),
        file: file("parameter/input.js"),
        span: span((9, 11), (9, 15)),
    },
    ParameterExpectedRow {
        id: node(162),
        kind: name("positional"),
        refinement: name("identifier"),
        name: name("a"),
        ordinal: number(0),
        owner: node(161),
        file: file("parameter/input.js"),
        span: span((10, 11), (10, 12)),
    },
    ParameterExpectedRow {
        id: node(265),
        kind: name("positional"),
        refinement: name("identifier"),
        name: name("name"),
        ordinal: number(0),
        owner: node(259),
        file: file("parameter/input.js"),
        span: span((15, 21), (15, 30)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
