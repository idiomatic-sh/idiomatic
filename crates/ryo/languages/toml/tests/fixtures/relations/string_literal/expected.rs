// The rows `relations/string_literal/input.toml` must give.

use ryo::languages::contract::{Expected, StringLiteralExpectedRow, expected_rows, file, name, node, span};

/// The rows, in the order the language gives them.
const ROWS: &[StringLiteralExpectedRow] = &[
    StringLiteralExpectedRow {
        occurrence: node(7),
        owner: node(1),
        value: name("t\tb"),
        file: file("string_literal/input.toml"),
        span: span((1, 5), (1, 11)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(15),
        owner: node(9),
        value: name("raw"),
        file: file("string_literal/input.toml"),
        span: span((2, 5), (2, 10)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(25),
        owner: node(23),
        value: name("x"),
        file: file("string_literal/input.toml"),
        span: span((3, 6), (3, 9)),
        import: false,
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
