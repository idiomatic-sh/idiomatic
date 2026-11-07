// The rows `relations/resolves/input.rs` must give.

use ryo::languages::contract::{Expected, ResolvesExpectedRow, expected_rows, file, name, node, span};

/// The rows, in the order the language gives them.
const ROWS: &[ResolvesExpectedRow] = &[
    ResolvesExpectedRow {
        occurrence: node(41),
        owner: node(21),
        name: name("Record"),
        definition: node(4),
        file: file("resolves/input.rs"),
        span: span((3, 28), (3, 34)),
    },
    ResolvesExpectedRow {
        occurrence: node(61),
        owner: node(21),
        name: name("Record"),
        definition: node(4),
        file: file("resolves/input.rs"),
        span: span((3, 47), (3, 53)),
    },
    ResolvesExpectedRow {
        occurrence: node(74),
        owner: node(21),
        name: name("T"),
        definition: node(28),
        file: file("resolves/input.rs"),
        span: span((3, 63), (3, 64)),
    },
    ResolvesExpectedRow {
        occurrence: node(108),
        owner: node(21),
        name: name("Local"),
        definition: node(116),
        file: file("resolves/input.rs"),
        span: span((3, 101), (3, 106)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
