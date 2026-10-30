// The rows `relations/assigns/input.js` must give.

use ryo::languages::contract::{AssignsExpectedRow, Expected, expected_rows, file, node, span};

/// The rows, in the order the language gives them.
const ROWS: &[AssignsExpectedRow] = &[
    AssignsExpectedRow {
        occurrence: node(4),
        binding: node(5),
        value: node(10),
        owner: node(0),
        file: file("assigns/input.js"),
        span: span((1, 18), (1, 25)),
    },
    AssignsExpectedRow {
        occurrence: node(23),
        binding: node(24),
        value: node(29),
        owner: node(0),
        file: file("assigns/input.js"),
        span: span((3, 10), (3, 17)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
