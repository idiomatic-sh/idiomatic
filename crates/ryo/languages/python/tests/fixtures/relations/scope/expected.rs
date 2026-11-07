// The rows `relations/scope/input.py` must give.

use ryo::languages::contract::{Expected, ScopeExpectedRow, expected_rows, file, node, span};

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(&[
        ScopeExpectedRow {
            parent: node(0),
            child: node(0),
            file: file("scope/input.py"),
            span: span((1, 1), (15, 1)),
        },
        ScopeExpectedRow {
            parent: node(0),
            child: node(1),
            file: file("scope/input.py"),
            span: span((1, 1), (8, 13)),
        },
        ScopeExpectedRow {
            parent: node(0),
            child: node(93),
            file: file("scope/input.py"),
            span: span((11, 11), (11, 39)),
        },
        ScopeExpectedRow {
            parent: node(0),
            child: node(132),
            file: file("scope/input.py"),
            span: span((12, 9), (12, 43)),
        },
        ScopeExpectedRow {
            parent: node(0),
            child: node(177),
            file: file("scope/input.py"),
            span: span((13, 13), (13, 40)),
        },
        ScopeExpectedRow {
            parent: node(0),
            child: node(207),
            file: file("scope/input.py"),
            span: span((14, 8), (14, 26)),
        },
        ScopeExpectedRow {
            parent: node(1),
            child: node(9),
            file: file("scope/input.py"),
            span: span((2, 5), (5, 22)),
        },
        ScopeExpectedRow {
            parent: node(1),
            child: node(72),
            file: file("scope/input.py"),
            span: span((7, 5), (8, 13)),
        },
        ScopeExpectedRow {
            parent: node(9),
            child: node(24),
            file: file("scope/input.py"),
            span: span((3, 9), (4, 41)),
        },
        ScopeExpectedRow {
            parent: node(24),
            child: node(38),
            file: file("scope/input.py"),
            span: span((4, 20), (4, 41)),
        },
    ])
}
