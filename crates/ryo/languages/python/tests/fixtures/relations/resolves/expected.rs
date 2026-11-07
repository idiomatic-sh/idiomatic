// The rows `relations/resolves/input.py` must give.

use ryo::languages::contract::{
    Expected, ResolvesExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[ResolvesExpectedRow] = &[
    ResolvesExpectedRow {
        occurrence: node(40),
        owner: node(27),
        name: name("Record"),
        definition: node(9),
        file: file("resolves/input.py"),
        span: span((7, 13), (7, 19)),
    },
    ResolvesExpectedRow {
        occurrence: node(53),
        owner: node(27),
        name: name("Record"),
        definition: node(9),
        file: file("resolves/input.py"),
        span: span((8, 17), (8, 23)),
    },
    ResolvesExpectedRow {
        occurrence: node(64),
        owner: node(27),
        name: name("Local"),
        definition: node(14),
        file: file("resolves/input.py"),
        span: span((9, 12), (9, 17)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
