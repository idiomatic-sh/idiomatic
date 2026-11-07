// The rows `relations/visibility/input.rs` must give.

use ryo::languages::contract::{
    Expected, VisibilityExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[VisibilityExpectedRow] = &[
    VisibilityExpectedRow {
        occurrence: node(2),
        declaration: node(1),
        text: name("pub"),
        file: file("visibility/input.rs"),
        span: span((1, 1), (1, 4)),
    },
    VisibilityExpectedRow {
        occurrence: node(14),
        declaration: node(13),
        text: name("pub(crate)"),
        file: file("visibility/input.rs"),
        span: span((2, 5), (2, 15)),
    },
    VisibilityExpectedRow {
        occurrence: node(50),
        declaration: node(49),
        text: name("pub(super)"),
        file: file("visibility/input.rs"),
        span: span((5, 1), (5, 11)),
    },
    VisibilityExpectedRow {
        occurrence: node(75),
        declaration: node(74),
        text: name("pub(in crate::a)"),
        file: file("visibility/input.rs"),
        span: span((6, 1), (6, 17)),
    },
    VisibilityExpectedRow {
        occurrence: node(106),
        declaration: node(105),
        text: name("pub"),
        file: file("visibility/input.rs"),
        span: span((7, 1), (7, 4)),
    },
    VisibilityExpectedRow {
        occurrence: node(116),
        declaration: node(115),
        text: name("pub"),
        file: file("visibility/input.rs"),
        span: span((7, 17), (7, 20)),
    },
    VisibilityExpectedRow {
        occurrence: node(136),
        declaration: node(154),
        text: name("pub"),
        file: file("visibility/input.rs"),
        span: span((8, 1), (8, 4)),
    },
    VisibilityExpectedRow {
        occurrence: node(136),
        declaration: node(161),
        text: name("pub"),
        file: file("visibility/input.rs"),
        span: span((8, 1), (8, 4)),
    },
    VisibilityExpectedRow {
        occurrence: node(170),
        declaration: node(169),
        text: name("pub"),
        file: file("visibility/input.rs"),
        span: span((9, 1), (9, 4)),
    },
    VisibilityExpectedRow {
        occurrence: node(192),
        declaration: node(191),
        text: name("pub"),
        file: file("visibility/input.rs"),
        span: span((10, 1), (10, 4)),
    },
    VisibilityExpectedRow {
        occurrence: node(214),
        declaration: node(213),
        text: name("pub"),
        file: file("visibility/input.rs"),
        span: span((11, 1), (11, 4)),
    },
    VisibilityExpectedRow {
        occurrence: node(227),
        declaration: node(226),
        text: name("pub"),
        file: file("visibility/input.rs"),
        span: span((12, 1), (12, 4)),
    },
    VisibilityExpectedRow {
        occurrence: node(240),
        declaration: node(239),
        text: name("pub"),
        file: file("visibility/input.rs"),
        span: span((13, 1), (13, 4)),
    },
    VisibilityExpectedRow {
        occurrence: node(258),
        declaration: node(257),
        text: name("pub"),
        file: file("visibility/input.rs"),
        span: span((14, 1), (14, 4)),
    },
    VisibilityExpectedRow {
        occurrence: node(282),
        declaration: node(281),
        text: name("pub"),
        file: file("visibility/input.rs"),
        span: span((15, 16), (15, 19)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
