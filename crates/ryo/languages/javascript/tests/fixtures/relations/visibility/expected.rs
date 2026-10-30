// The rows `relations/visibility/input.js` must give.

use ryo::languages::contract::{
    Expected, VisibilityExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[VisibilityExpectedRow] = &[
    VisibilityExpectedRow {
        occurrence: node(2),
        declaration: node(4),
        text: name("export"),
        file: file("visibility/input.js"),
        span: span((1, 1), (1, 7)),
    },
    VisibilityExpectedRow {
        occurrence: node(17),
        declaration: node(21),
        text: name("export default"),
        file: file("visibility/input.js"),
        span: span((2, 1), (2, 15)),
    },
    VisibilityExpectedRow {
        occurrence: node(32),
        declaration: node(37),
        text: name("export"),
        file: file("visibility/input.js"),
        span: span((3, 1), (3, 7)),
    },
    VisibilityExpectedRow {
        occurrence: node(32),
        declaration: node(47),
        text: name("export"),
        file: file("visibility/input.js"),
        span: span((3, 1), (3, 7)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
