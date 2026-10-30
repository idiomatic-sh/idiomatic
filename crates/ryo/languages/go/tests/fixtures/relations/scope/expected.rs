// The rows `relations/scope/input.go` must give.

use ryo::languages::contract::{Expected, ScopeExpectedRow, expected_rows, file, node, span};

/// The rows, in the order the language gives them.
const ROWS: &[ScopeExpectedRow] = &[
    ScopeExpectedRow {
        parent: node(0),
        child: node(0),
        file: file("scope/input.go"),
        span: span((1, 1), (14, 1)),
    },
    ScopeExpectedRow {
        parent: node(0),
        child: node(19),
        file: file("scope/input.go"),
        span: span((5, 1), (11, 2)),
    },
    ScopeExpectedRow {
        parent: node(0),
        child: node(87),
        file: file("scope/input.go"),
        span: span((13, 1), (13, 18)),
    },
    ScopeExpectedRow {
        parent: node(19),
        child: node(38),
        file: file("scope/input.go"),
        span: span((6, 7), (9, 3)),
    },
    ScopeExpectedRow {
        parent: node(38),
        child: node(54),
        file: file("scope/input.go"),
        span: span((7, 8), (7, 17)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
