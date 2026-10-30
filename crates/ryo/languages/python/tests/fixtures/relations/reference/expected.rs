// The rows `relations/reference/input.py` must give.

use ryo::languages::contract::{
    Expected, ReferenceExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[ReferenceExpectedRow] = &[
    ReferenceExpectedRow {
        occurrence: node(16),
        owner: node(0),
        target: name("os.path.join"),
        file: file("reference/input.py"),
        span: span((3, 9), (3, 21)),
    },
    ReferenceExpectedRow {
        occurrence: node(28),
        owner: node(0),
        target: name("base"),
        file: file("reference/input.py"),
        span: span((3, 22), (3, 26)),
    },
    ReferenceExpectedRow {
        occurrence: node(32),
        owner: node(0),
        target: name("name"),
        file: file("reference/input.py"),
        span: span((3, 28), (3, 32)),
    },
    ReferenceExpectedRow {
        occurrence: node(38),
        owner: node(0),
        target: name("items.append"),
        file: file("reference/input.py"),
        span: span((4, 1), (4, 13)),
    },
    ReferenceExpectedRow {
        occurrence: node(46),
        owner: node(0),
        target: name("total"),
        file: file("reference/input.py"),
        span: span((4, 14), (4, 19)),
    },
    ReferenceExpectedRow {
        occurrence: node(52),
        owner: node(0),
        target: name("obj"),
        file: file("reference/input.py"),
        span: span((5, 1), (5, 4)),
    },
    ReferenceExpectedRow {
        occurrence: node(60),
        owner: node(0),
        target: name("value"),
        file: file("reference/input.py"),
        span: span((5, 12), (5, 17)),
    },
    ReferenceExpectedRow {
        occurrence: node(67),
        owner: node(0),
        target: name("cache"),
        file: file("reference/input.py"),
        span: span((6, 5), (6, 10)),
    },
    ReferenceExpectedRow {
        occurrence: node(70),
        owner: node(0),
        target: name("key"),
        file: file("reference/input.py"),
        span: span((6, 11), (6, 14)),
    },
    ReferenceExpectedRow {
        occurrence: node(75),
        owner: node(0),
        target: name("stale"),
        file: file("reference/input.py"),
        span: span((6, 17), (6, 22)),
    },
    ReferenceExpectedRow {
        occurrence: node(80),
        owner: node(0),
        target: name("print"),
        file: file("reference/input.py"),
        span: span((7, 1), (7, 6)),
    },
    ReferenceExpectedRow {
        occurrence: node(85),
        owner: node(0),
        target: name("len"),
        file: file("reference/input.py"),
        span: span((7, 7), (7, 10)),
    },
    ReferenceExpectedRow {
        occurrence: node(89),
        owner: node(0),
        target: name("items"),
        file: file("reference/input.py"),
        span: span((7, 11), (7, 16)),
    },
    ReferenceExpectedRow {
        occurrence: node(94),
        owner: node(0),
        target: name("__name__"),
        file: file("reference/input.py"),
        span: span((7, 19), (7, 27)),
    },
    ReferenceExpectedRow {
        occurrence: node(107),
        owner: node(0),
        target: name("factory"),
        file: file("reference/input.py"),
        span: span((8, 10), (8, 17)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
