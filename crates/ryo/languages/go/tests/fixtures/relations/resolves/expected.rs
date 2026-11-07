// The rows `relations/resolves/input.go` must give.

use ryo::languages::contract::{
    Expected, ResolvesExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[ResolvesExpectedRow] = &[
    ResolvesExpectedRow {
        occurrence: node(57),
        owner: node(29),
        name: name("Local"),
        definition: node(16),
        file: file("resolves/input.go"),
        span: span((9, 11), (9, 16)),
    },
    ResolvesExpectedRow {
        occurrence: node(66),
        owner: node(29),
        name: name("Local"),
        definition: node(16),
        file: file("resolves/input.go"),
        span: span((10, 10), (10, 15)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
