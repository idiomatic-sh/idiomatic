// The rows `relations/string_literal/input.go` must give.

use ryo::languages::contract::{
    Expected, StringLiteralExpectedRow, expected_rows, file, name, node, span, unknown,
};

/// The rows, in the order the language gives them.
const ROWS: &[StringLiteralExpectedRow] = &[
    StringLiteralExpectedRow {
        occurrence: node(10),
        owner: node(0),
        value: name("fmt"),
        file: file("string_literal/input.go"),
        span: span((3, 8), (3, 13)),
        import: true,
    },
    StringLiteralExpectedRow {
        occurrence: node(22),
        owner: node(0),
        value: name("plain"),
        file: file("string_literal/input.go"),
        span: span((5, 13), (5, 20)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(34),
        owner: node(0),
        value: name("tab\there é A A"),
        file: file("string_literal/input.go"),
        span: span((6, 15), (6, 39)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(46),
        owner: node(0),
        value: name("raw \\n text"),
        file: file("string_literal/input.go"),
        span: span((7, 11), (7, 24)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(58),
        owner: node(0),
        value: unknown("not-text"),
        file: file("string_literal/input.go"),
        span: span((8, 13), (8, 19)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(93),
        owner: node(0),
        value: name("json:\"name\""),
        file: file("string_literal/input.go"),
        span: span((12, 14), (12, 27)),
        import: false,
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
