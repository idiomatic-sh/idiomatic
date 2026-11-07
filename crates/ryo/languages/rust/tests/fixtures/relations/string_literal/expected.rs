// The rows `relations/string_literal/input.rs` must give.

use ryo::languages::contract::{
    Expected, StringLiteralExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[StringLiteralExpectedRow] = &[
    StringLiteralExpectedRow {
        occurrence: node(18),
        owner: node(0),
        value: name("value"),
        file: file("string_literal/input.rs"),
        span: span((1, 21), (1, 28)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(44),
        owner: node(22),
        value: name("a\tb"),
        file: file("string_literal/input.rs"),
        span: span((3, 23), (3, 29)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(62),
        owner: node(0),
        value: name("other.rs"),
        file: file("string_literal/input.rs"),
        span: span((5, 10), (5, 20)),
        import: true,
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
