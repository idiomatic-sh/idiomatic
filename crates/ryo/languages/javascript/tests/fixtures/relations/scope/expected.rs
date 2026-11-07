// The rows `relations/scope/input.js` must give.

use ryo::languages::contract::{Expected, ScopeExpectedRow, expected_rows, file, node, span};

/// The rows, in the order the language gives them.
const ROWS: &[ScopeExpectedRow] = &[
    ScopeExpectedRow {
        parent: node(0),
        child: node(0),
        file: file("scope/input.js"),
        span: span((1, 1), (18, 1)),
    },
    ScopeExpectedRow {
        parent: node(0),
        child: node(1),
        file: file("scope/input.js"),
        span: span((1, 1), (10, 2)),
    },
    ScopeExpectedRow {
        parent: node(0),
        child: node(109),
        file: file("scope/input.js"),
        span: span((11, 1), (15, 2)),
    },
    ScopeExpectedRow {
        parent: node(0),
        child: node(135),
        file: file("scope/input.js"),
        span: span((16, 1), (16, 28)),
    },
    ScopeExpectedRow {
        parent: node(0),
        child: node(156),
        file: file("scope/input.js"),
        span: span((17, 1), (17, 28)),
    },
    ScopeExpectedRow {
        parent: node(1),
        child: node(11),
        file: file("scope/input.js"),
        span: span((1, 19), (10, 2)),
    },
    ScopeExpectedRow {
        parent: node(11),
        child: node(14),
        file: file("scope/input.js"),
        span: span((2, 3), (8, 4)),
    },
    ScopeExpectedRow {
        parent: node(11),
        child: node(96),
        file: file("scope/input.js"),
        span: span((9, 10), (9, 18)),
    },
    ScopeExpectedRow {
        parent: node(14),
        child: node(47),
        file: file("scope/input.js"),
        span: span((2, 31), (8, 4)),
    },
    ScopeExpectedRow {
        parent: node(47),
        child: node(53),
        file: file("scope/input.js"),
        span: span((3, 9), (5, 6)),
    },
    ScopeExpectedRow {
        parent: node(47),
        child: node(68),
        file: file("scope/input.js"),
        span: span((5, 7), (7, 6)),
    },
    ScopeExpectedRow {
        parent: node(68),
        child: node(76),
        file: file("scope/input.js"),
        span: span((5, 21), (7, 6)),
    },
    ScopeExpectedRow {
        parent: node(96),
        child: node(102),
        file: file("scope/input.js"),
        span: span((9, 16), (9, 18)),
    },
    ScopeExpectedRow {
        parent: node(109),
        child: node(118),
        file: file("scope/input.js"),
        span: span((12, 3), (14, 4)),
    },
    ScopeExpectedRow {
        parent: node(135),
        child: node(152),
        file: file("scope/input.js"),
        span: span((16, 26), (16, 28)),
    },
    ScopeExpectedRow {
        parent: node(156),
        child: node(173),
        file: file("scope/input.js"),
        span: span((17, 26), (17, 28)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
