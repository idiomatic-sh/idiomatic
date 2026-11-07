// Independent full rows include normalized payload and exact literal source identity.

use ryo::languages::contract::{
    Expected, NumericLiteralExpectedRow, expected_rows, file, name, node, span,
};

const ROWS: &[NumericLiteralExpectedRow] = &[
    NumericLiteralExpectedRow {
        id: node(2),
        kind: name("integer"),
        value: name("16"),
        file: file("numeric_literal/input.py"),
        span: span((1, 1), (1, 5)),
    },
    NumericLiteralExpectedRow {
        id: node(6),
        kind: name("integer"),
        value: name("9007199254740993"),
        file: file("numeric_literal/input.py"),
        span: span((2, 1), (2, 17)),
    },
    NumericLiteralExpectedRow {
        id: node(10),
        kind: name("float"),
        value: name("1.0"),
        file: file("numeric_literal/input.py"),
        span: span((3, 1), (3, 4)),
    },
];

pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
