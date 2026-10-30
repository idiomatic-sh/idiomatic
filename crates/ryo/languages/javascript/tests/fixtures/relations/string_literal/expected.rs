// The rows `relations/string_literal/input.js` must give.

use ryo::languages::contract::{
    Expected, StringLiteralExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[StringLiteralExpectedRow] = &[
    StringLiteralExpectedRow {
        occurrence: node(24),
        owner: node(0),
        value: name("module"),
        file: file("string_literal/input.js"),
        span: span((2, 34), (2, 42)),
        import: true,
    },
    StringLiteralExpectedRow {
        occurrence: node(43),
        owner: node(31),
        value: name("world"),
        file: file("string_literal/input.js"),
        span: span((3, 30), (3, 37)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(66),
        owner: node(53),
        value: name("hello\n"),
        file: file("string_literal/input.js"),
        span: span((4, 14), (4, 23)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(300),
        owner: node(0),
        value: name("escaped"),
        file: file("string_literal/input.js"),
        span: span((16, 35), (16, 44)),
        import: true,
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
