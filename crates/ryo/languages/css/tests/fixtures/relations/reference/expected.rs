// The rows `relations/reference/input.css` must give.

use ryo::languages::contract::{Expected, ReferenceExpectedRow, expected_rows, file, name, node, span};

/// The rows, in the order the language gives them.
const ROWS: &[ReferenceExpectedRow] = &[
    ReferenceExpectedRow {
        occurrence: node(11),
        owner: node(7),
        target: name("img/hero.png"),
        file: file("reference/input.css"),
        span: span((1, 21), (1, 41)),
    },
    ReferenceExpectedRow {
        occurrence: node(16),
        owner: node(7),
        target: name("data:image/png;base64,AA=="),
        file: file("reference/input.css"),
        span: span((1, 53), (1, 86)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
