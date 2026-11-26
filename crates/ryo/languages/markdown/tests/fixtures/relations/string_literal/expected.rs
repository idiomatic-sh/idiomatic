// The rows `relations/string_literal/input.md` must give.

use ryo::languages::contract::{
    Expected, StringLiteralExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[StringLiteralExpectedRow] = &[
    StringLiteralExpectedRow {
        occurrence: node(3),
        owner: node(1),
        value: name("cargo test"),
        file: file("string_literal/input.md"),
        span: span((1, 5), (1, 17)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(7),
        owner: node(0),
        value: name("cargo test --all-features\n"),
        file: file("string_literal/input.md"),
        span: span((3, 1), (5, 4)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(12),
        owner: node(0),
        value: name("indented code\n"),
        file: file("string_literal/input.md"),
        span: span((7, 5), (8, 1)),
        import: false,
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
