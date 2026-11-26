// The rows `relations/scope/input.md` must give.

use ryo::languages::contract::{Expected, ScopeExpectedRow, expected_rows, file, node, span};

/// The rows, in the order the language gives them.
const ROWS: &[ScopeExpectedRow] = &[
    ScopeExpectedRow {
        parent: node(0),
        child: node(0),
        file: file("scope/input.md"),
        span: span((1, 1), (14, 1)),
    },
    ScopeExpectedRow {
        parent: node(0),
        child: node(6),
        file: file("scope/input.md"),
        span: span((3, 1), (11, 1)),
    },
    ScopeExpectedRow {
        parent: node(0),
        child: node(30),
        file: file("scope/input.md"),
        span: span((11, 1), (14, 1)),
    },
    ScopeExpectedRow {
        parent: node(6),
        child: node(12),
        file: file("scope/input.md"),
        span: span((5, 1), (9, 1)),
    },
    ScopeExpectedRow {
        parent: node(6),
        child: node(24),
        file: file("scope/input.md"),
        span: span((9, 1), (11, 1)),
    },
    ScopeExpectedRow {
        parent: node(12),
        child: node(18),
        file: file("scope/input.md"),
        span: span((7, 1), (9, 1)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
