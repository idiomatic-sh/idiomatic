// The rows `relations/inherits/input.ts` must give.

use ryo::languages::contract::{
    Expected, InheritsExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[InheritsExpectedRow] = &[InheritsExpectedRow {
    occurrence: node(9),
    declaration: node(1),
    target: name("Base"),
    file: file("inherits/input.ts"),
    span: span((1, 17), (1, 21)),
}];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
