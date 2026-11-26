// The rows `relations/string_literal/input.json` must give.

use ryo::languages::contract::{
    Expected, StringLiteralExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[StringLiteralExpectedRow] = &[
    StringLiteralExpectedRow {
        occurrence: node(7),
        owner: node(3),
        value: name("v"),
        file: file("string_literal/input.json"),
        span: span((1, 7), (1, 10)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(16),
        owner: node(14),
        value: name("a"),
        file: file("string_literal/input.json"),
        span: span((1, 21), (1, 24)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(19),
        owner: node(14),
        value: name("b\n"),
        file: file("string_literal/input.json"),
        span: span((1, 26), (1, 31)),
        import: false,
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
