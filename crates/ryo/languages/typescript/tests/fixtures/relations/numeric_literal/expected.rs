// The rows `relations/numeric_literal/input.ts` must give.

use ryo::languages::contract::{
    Expected, NumericLiteralExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[NumericLiteralExpectedRow] = &[
    NumericLiteralExpectedRow {
        id: node(10),
        kind: name("float"),
        value: name("16.0"),
        file: file("numeric_literal/input.ts"),
        span: span((1, 11), (1, 15)),
    },
    NumericLiteralExpectedRow {
        id: node(23),
        kind: name("float"),
        value: name("1500.0"),
        file: file("numeric_literal/input.ts"),
        span: span((2, 11), (2, 16)),
    },
    NumericLiteralExpectedRow {
        id: node(36),
        kind: name("integer"),
        value: name("10"),
        file: file("numeric_literal/input.ts"),
        span: span((3, 11), (3, 14)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
