// The rows `relations/defines/input.html` must give.

use ryo::languages::contract::{
    DefinesExpectedRow, Expected, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[DefinesExpectedRow] = &[
    DefinesExpectedRow {
        id: node(21),
        kind: name("section"),
        refinement: name("element"),
        name: name("Intro part"),
        owner: node(0),
        file: file("defines/input.html"),
        span: span((1, 48), (1, 281)),
    },
    DefinesExpectedRow {
        id: node(24),
        kind: name("label"),
        refinement: name("attribute"),
        name: name("intro"),
        owner: node(21),
        file: file("defines/input.html"),
        span: span((1, 57), (1, 67)),
    },
    DefinesExpectedRow {
        id: node(55),
        kind: name("section"),
        refinement: name("element"),
        name: name("Child"),
        owner: node(21),
        file: file("defines/input.html"),
        span: span((1, 162), (1, 195)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
