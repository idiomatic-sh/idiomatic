// The rows `relations/assigns/input.ts` must give.

use ryo::languages::contract::{AssignsExpectedRow, Expected, expected_rows, file, node, span};

/// The rows, in the order the language gives them.
const ROWS: &[AssignsExpectedRow] = &[AssignsExpectedRow {
    occurrence: node(4),
    binding: node(5),
    value: node(10),
    owner: node(0),
    file: file("assigns/input.ts"),
    span: span((1, 11), (1, 15)),
}];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
