// The rows `relations/numeric_literal/input.rs` must give.

use ryo::languages::contract::{
    Expected, NumericLiteralExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[NumericLiteralExpectedRow] = &[
    NumericLiteralExpectedRow {
        id: node(18),
        kind: name("integer"),
        value: name("3"),
        exact_digits: true,
        file: file("numeric_literal/input.rs"),
        span: span((1, 21), (1, 22)),
    },
    NumericLiteralExpectedRow {
        id: node(26),
        kind: name("integer"),
        value: name("16"),
        exact_digits: true,
        file: file("numeric_literal/input.rs"),
        span: span((1, 27), (1, 29)),
    },
    NumericLiteralExpectedRow {
        id: node(30),
        kind: name("integer"),
        value: name("16"),
        exact_digits: true,
        file: file("numeric_literal/input.rs"),
        span: span((1, 31), (1, 35)),
    },
    NumericLiteralExpectedRow {
        id: node(34),
        kind: name("integer"),
        value: name("16"),
        exact_digits: true,
        file: file("numeric_literal/input.rs"),
        span: span((1, 37), (1, 45)),
    },
    NumericLiteralExpectedRow {
        id: node(54),
        kind: name("float"),
        value: name("0.10000000149011612"),
        exact_digits: true,
        file: file("numeric_literal/input.rs"),
        span: span((3, 20), (3, 26)),
    },
    NumericLiteralExpectedRow {
        id: node(124),
        kind: name("float"),
        value: name("1000.0"),
        exact_digits: true,
        file: file("numeric_literal/input.rs"),
        span: span((6, 33), (6, 36)),
    },
    NumericLiteralExpectedRow {
        id: node(152),
        kind: name("float"),
        value: name("0.3"),
        exact_digits: false,
        file: file("numeric_literal/input.rs"),
        span: span((9, 22), (9, 44)),
    },
    NumericLiteralExpectedRow {
        id: node(164),
        kind: name("integer"),
        value: name("1"),
        exact_digits: true,
        file: file("numeric_literal/input.rs"),
        span: span((6, 18), (6, 19)),
    },
    NumericLiteralExpectedRow {
        id: node(168),
        kind: name("integer"),
        value: name("9"),
        exact_digits: true,
        file: file("numeric_literal/input.rs"),
        span: span((6, 22), (6, 23)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
