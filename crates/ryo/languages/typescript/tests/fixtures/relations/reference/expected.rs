// The rows `relations/reference/input.ts` must give.

use ryo::languages::contract::{
    Expected, ReferenceExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[ReferenceExpectedRow] = &[
    ReferenceExpectedRow {
        occurrence: node(2),
        owner: node(0),
        target: name("x"),
        file: file("reference/input.ts"),
        span: span((1, 1), (1, 2)),
    },
    ReferenceExpectedRow {
        occurrence: node(8),
        owner: node(0),
        target: name("foo"),
        file: file("reference/input.ts"),
        span: span((2, 1), (2, 4)),
    },
    ReferenceExpectedRow {
        occurrence: node(11),
        owner: node(0),
        target: name("bar"),
        file: file("reference/input.ts"),
        span: span((2, 5), (2, 8)),
    },
    ReferenceExpectedRow {
        occurrence: node(18),
        owner: node(0),
        target: name("foo"),
        file: file("reference/input.ts"),
        span: span((3, 1), (3, 4)),
    },
    ReferenceExpectedRow {
        occurrence: node(21),
        owner: node(0),
        target: name("obj.value"),
        file: file("reference/input.ts"),
        span: span((3, 5), (3, 14)),
    },
    ReferenceExpectedRow {
        occurrence: node(40),
        owner: node(35),
        target: name("Base"),
        file: file("reference/input.ts"),
        span: span((4, 30), (4, 34)),
    },
    ReferenceExpectedRow {
        occurrence: node(61),
        owner: node(56),
        target: name("Parent"),
        file: file("reference/input.ts"),
        span: span((5, 25), (5, 31)),
    },
    ReferenceExpectedRow {
        occurrence: node(80),
        owner: node(69),
        target: name("key"),
        file: file("reference/input.ts"),
        span: span((6, 12), (6, 15)),
    },
    ReferenceExpectedRow {
        occurrence: node(105),
        owner: node(0),
        target: name("computed"),
        file: file("reference/input.ts"),
        span: span((7, 19), (7, 27)),
    },
    ReferenceExpectedRow {
        occurrence: node(110),
        owner: node(0),
        target: name("value"),
        file: file("reference/input.ts"),
        span: span((7, 30), (7, 35)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
