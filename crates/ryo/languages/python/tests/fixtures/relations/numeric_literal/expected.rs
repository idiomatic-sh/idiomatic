// Independent full rows include normalized payload and exact literal source identity.

use ryo::languages::contract::{
    Expected, NumericLiteralExpectedRow, expected_rows, file, name, node, span,
};

const ROWS: &[NumericLiteralExpectedRow] = &[
    NumericLiteralExpectedRow {
        id: node(2),
        kind: name("integer"),
        value: name("16"),
        exact_digits: true,
        file: file("numeric_literal/input.py"),
        span: span((1, 1), (1, 5)),
    },
    NumericLiteralExpectedRow {
        id: node(6),
        kind: name("integer"),
        value: name("9007199254740993"),
        exact_digits: true,
        file: file("numeric_literal/input.py"),
        span: span((2, 1), (2, 17)),
    },
    NumericLiteralExpectedRow {
        id: node(10),
        kind: name("float"),
        value: name("1.0"),
        exact_digits: true,
        file: file("numeric_literal/input.py"),
        span: span((3, 1), (3, 4)),
    },
    NumericLiteralExpectedRow {
        id: node(22),
        kind: name("float"),
        value: name("9007199254740992.0"),
        exact_digits: false,
        file: file("numeric_literal/input.py"),
        span: span((6, 1), (6, 19)),
    },
    NumericLiteralExpectedRow {
        id: node(26),
        kind: name("float"),
        value: name("10500.0"),
        exact_digits: true,
        file: file("numeric_literal/input.py"),
        span: span((7, 1), (7, 8)),
    },
];

pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
