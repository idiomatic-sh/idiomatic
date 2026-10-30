// The rows `relations/visibility/input.ts` must give.

use ryo::languages::contract::{
    Expected, VisibilityExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[VisibilityExpectedRow] = &[
    VisibilityExpectedRow {
        occurrence: node(2),
        declaration: node(4),
        text: name("export"),
        file: file("visibility/input.ts"),
        span: span((1, 1), (1, 7)),
    },
    VisibilityExpectedRow {
        occurrence: node(22),
        declaration: node(26),
        text: name("export default"),
        file: file("visibility/input.ts"),
        span: span((2, 1), (2, 15)),
    },
    VisibilityExpectedRow {
        occurrence: node(36),
        declaration: node(35),
        text: name("private"),
        file: file("visibility/input.ts"),
        span: span((3, 3), (3, 10)),
    },
    VisibilityExpectedRow {
        occurrence: node(48),
        declaration: node(47),
        text: name("protected"),
        file: file("visibility/input.ts"),
        span: span((4, 3), (4, 12)),
    },
    VisibilityExpectedRow {
        occurrence: node(60),
        declaration: node(59),
        text: name("public"),
        file: file("visibility/input.ts"),
        span: span((5, 3), (5, 9)),
    },
    VisibilityExpectedRow {
        occurrence: node(98),
        declaration: node(103),
        text: name("export"),
        file: file("visibility/input.ts"),
        span: span((8, 1), (8, 7)),
    },
    VisibilityExpectedRow {
        occurrence: node(98),
        declaration: node(113),
        text: name("export"),
        file: file("visibility/input.ts"),
        span: span((8, 1), (8, 7)),
    },
    VisibilityExpectedRow {
        occurrence: node(124),
        declaration: node(126),
        text: name("export"),
        file: file("visibility/input.ts"),
        span: span((9, 1), (9, 7)),
    },
    VisibilityExpectedRow {
        occurrence: node(147),
        declaration: node(149),
        text: name("export"),
        file: file("visibility/input.ts"),
        span: span((10, 1), (10, 7)),
    },
    VisibilityExpectedRow {
        occurrence: node(162),
        declaration: node(164),
        text: name("export"),
        file: file("visibility/input.ts"),
        span: span((11, 1), (11, 7)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
