// The rows `relations/defines/input.md` must give.

use ryo::languages::contract::{
    DefinesExpectedRow, Expected, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[DefinesExpectedRow] = &[
    DefinesExpectedRow {
        id: node(2),
        kind: name("section"),
        refinement: name("heading"),
        name: name("Guide"),
        owner: node(0),
        file: file("defines/input.md"),
        span: span((1, 1), (2, 1)),
    },
    DefinesExpectedRow {
        id: node(9),
        kind: name("label"),
        refinement: name("html"),
        name: name("top"),
        owner: node(2),
        file: file("defines/input.md"),
        span: span((3, 12), (3, 24)),
    },
    DefinesExpectedRow {
        id: node(20),
        kind: name("section"),
        refinement: name("heading"),
        name: name("Setup"),
        owner: node(2),
        file: file("defines/input.md"),
        span: span((5, 1), (6, 1)),
    },
    DefinesExpectedRow {
        id: node(26),
        kind: name("section"),
        refinement: name("heading"),
        name: name("Steps"),
        owner: node(20),
        file: file("defines/input.md"),
        span: span((7, 1), (8, 1)),
    },
    DefinesExpectedRow {
        id: node(38),
        kind: name("section"),
        refinement: name("heading"),
        name: name("Usage"),
        owner: node(2),
        file: file("defines/input.md"),
        span: span((11, 1), (12, 1)),
    },
    DefinesExpectedRow {
        id: node(43),
        kind: name("label"),
        refinement: name("definition"),
        name: name("ref"),
        owner: node(38),
        file: file("defines/input.md"),
        span: span((13, 1), (13, 31)),
    },
    DefinesExpectedRow {
        id: node(45),
        kind: name("label"),
        refinement: name("footnote-definition"),
        name: name("1"),
        owner: node(38),
        file: file("defines/input.md"),
        span: span((15, 1), (17, 1)),
    },
    DefinesExpectedRow {
        id: node(52),
        kind: name("section"),
        refinement: name("heading"),
        name: name("The run step"),
        owner: node(2),
        file: file("defines/input.md"),
        span: span((17, 1), (18, 1)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
