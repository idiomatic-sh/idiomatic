// The rows `relations/annotates/input.ts` must give.

use ryo::languages::contract::{
    AnnotatesExpectedRow, Expected, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[AnnotatesExpectedRow] = &[AnnotatesExpectedRow {
    occurrence: node(7),
    owner: node(5),
    annotation: name("string"),
    file: file("annotates/input.ts"),
    span: span((1, 8), (1, 14)),
}];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
