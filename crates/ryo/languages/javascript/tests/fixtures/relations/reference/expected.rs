// The rows `relations/reference/input.js` must give.

use ryo::languages::contract::{
    Expected, ReferenceExpectedRow, expected_rows, file, name, node, span, unknown,
};

/// The rows, in the order the language gives them.
const ROWS: &[ReferenceExpectedRow] = &[
    ReferenceExpectedRow {
        occurrence: node(27),
        owner: node(0),
        target: name("read"),
        file: file("reference/input.js"),
        span: span((2, 14), (2, 18)),
    },
    ReferenceExpectedRow {
        occurrence: node(31),
        owner: node(0),
        target: name("path.join"),
        file: file("reference/input.js"),
        span: span((2, 19), (2, 28)),
    },
    ReferenceExpectedRow {
        occurrence: node(38),
        owner: node(0),
        target: name("root"),
        file: file("reference/input.js"),
        span: span((2, 29), (2, 33)),
    },
    ReferenceExpectedRow {
        occurrence: node(50),
        owner: node(0),
        target: name("console.log"),
        file: file("reference/input.js"),
        span: span((3, 1), (3, 12)),
    },
    ReferenceExpectedRow {
        occurrence: node(57),
        owner: node(0),
        target: name("data.length"),
        file: file("reference/input.js"),
        span: span((3, 13), (3, 24)),
    },
    ReferenceExpectedRow {
        occurrence: node(65),
        owner: node(0),
        target: unknown("dynamic-or-anonymous"),
        file: file("reference/input.js"),
        span: span((3, 26), (3, 35)),
    },
    ReferenceExpectedRow {
        occurrence: node(66),
        owner: node(0),
        target: name("data"),
        file: file("reference/input.js"),
        span: span((3, 26), (3, 30)),
    },
    ReferenceExpectedRow {
        occurrence: node(69),
        owner: node(0),
        target: name("key"),
        file: file("reference/input.js"),
        span: span((3, 31), (3, 34)),
    },
    ReferenceExpectedRow {
        occurrence: node(91),
        owner: node(85),
        target: name("item.name"),
        file: file("reference/input.js"),
        span: span((5, 10), (5, 19)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
