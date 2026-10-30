// The rows `relations/scope/input.rs` must give.

use ryo::languages::contract::{Expected, ScopeExpectedRow, expected_rows, file, node, span};

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(&[
        ScopeExpectedRow {
            parent: node(0),
            child: node(0),
            file: file("scope/input.rs"),
            span: span((1, 1), (12, 1)),
        },
        ScopeExpectedRow {
            parent: node(0),
            child: node(1),
            file: file("scope/input.rs"),
            span: span((1, 1), (1, 29)),
        },
        ScopeExpectedRow {
            parent: node(0),
            child: node(30),
            file: file("scope/input.rs"),
            span: span((3, 1), (3, 32)),
        },
        ScopeExpectedRow {
            parent: node(0),
            child: node(74),
            file: file("scope/input.rs"),
            span: span((5, 1), (5, 25)),
        },
        ScopeExpectedRow {
            parent: node(0),
            child: node(106),
            file: file("scope/input.rs"),
            span: span((9, 1), (9, 31)),
        },
        ScopeExpectedRow {
            parent: node(0),
            child: node(138),
            file: file("scope/input.rs"),
            span: span((11, 1), (11, 32)),
        },
        ScopeExpectedRow {
            parent: node(1),
            child: node(14),
            file: file("scope/input.rs"),
            span: span((1, 14), (1, 27)),
        },
        ScopeExpectedRow {
            parent: node(30),
            child: node(43),
            file: file("scope/input.rs"),
            span: span((3, 10), (3, 30)),
        },
        ScopeExpectedRow {
            parent: node(43),
            child: node(56),
            file: file("scope/input.rs"),
            span: span((3, 19), (3, 28)),
        },
        ScopeExpectedRow {
            parent: node(74),
            child: node(83),
            file: file("scope/input.rs"),
            span: span((5, 9), (5, 23)),
        },
        ScopeExpectedRow {
            parent: node(106),
            child: node(118),
            file: file("scope/input.rs"),
            span: span((9, 10), (9, 29)),
        },
        ScopeExpectedRow {
            parent: node(138),
            child: node(147),
            file: file("scope/input.rs"),
            span: span((11, 11), (11, 30)),
        },
    ])
}
