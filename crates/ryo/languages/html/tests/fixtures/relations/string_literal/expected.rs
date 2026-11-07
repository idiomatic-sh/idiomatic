// The rows `relations/string_literal/input.html` must give.

use ryo::languages::contract::{
    Expected, StringLiteralExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[StringLiteralExpectedRow] = &[
    StringLiteralExpectedRow {
        occurrence: node(24),
        owner: node(21),
        value: name("intro"),
        file: file("string_literal/input.html"),
        span: span((1, 57), (1, 67)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(30),
        owner: node(27),
        value: name("A & B"),
        file: file("string_literal/input.html"),
        span: span((1, 71), (1, 88)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(38),
        owner: node(35),
        value: name("guide.html#start"),
        file: file("string_literal/input.html"),
        span: span((1, 98), (1, 121)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(48),
        owner: node(45),
        value: name("a.png"),
        file: file("string_literal/input.html"),
        span: span((1, 141), (1, 152)),
        import: false,
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
