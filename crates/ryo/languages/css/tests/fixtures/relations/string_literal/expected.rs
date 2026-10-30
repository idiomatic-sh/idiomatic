// The rows `relations/string_literal/input.css` must give.

use ryo::languages::contract::{Expected, StringLiteralExpectedRow, expected_rows, file, name, node, span};

/// The rows, in the order the language gives them.
const ROWS: &[StringLiteralExpectedRow] = &[
    StringLiteralExpectedRow {
        occurrence: node(4),
        owner: node(1),
        value: name("base.css"),
        file: file("string_literal/input.css"),
        span: span((1, 9), (1, 19)),
        import: true,
    },
    StringLiteralExpectedRow {
        occurrence: node(16),
        owner: node(12),
        value: name("xA"),
        file: file("string_literal/input.css"),
        span: span((2, 14), (2, 20)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(23),
        owner: node(19),
        value: name("Open Sans"),
        file: file("string_literal/input.css"),
        span: span((2, 35), (2, 46)),
        import: false,
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
