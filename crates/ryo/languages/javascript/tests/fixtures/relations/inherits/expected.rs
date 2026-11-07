// The rows `relations/inherits/input.js` must give.

use ryo::languages::contract::{
    Expected, InheritsExpectedRow, expected_rows, file, name, node, span, unknown,
};

/// The rows, in the order the language gives them.
const ROWS: &[InheritsExpectedRow] = &[
    InheritsExpectedRow {
        occurrence: node(19),
        declaration: node(11),
        target: name("Base"),
        file: file("inherits/input.js"),
        span: span((2, 21), (2, 25)),
    },
    InheritsExpectedRow {
        occurrence: node(34),
        declaration: node(26),
        target: name("tool.Base"),
        file: file("inherits/input.js"),
        span: span((3, 20), (3, 29)),
    },
    InheritsExpectedRow {
        occurrence: node(59),
        declaration: node(54),
        target: unknown("dynamic-or-anonymous"),
        file: file("inherits/input.js"),
        span: span((4, 29), (4, 38)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
