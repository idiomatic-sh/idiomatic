// The rows `relations/string_literal/input.ts` must give.

use ryo::languages::contract::{
    Expected, StringLiteralExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[StringLiteralExpectedRow] = &[
    StringLiteralExpectedRow {
        occurrence: node(10),
        owner: node(0),
        value: name("hi\n"),
        file: file("string_literal/input.ts"),
        span: span((1, 11), (1, 17)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(23),
        owner: node(0),
        value: name("dep"),
        file: file("string_literal/input.ts"),
        span: span((2, 15), (2, 20)),
        import: true,
    },
    StringLiteralExpectedRow {
        occurrence: node(39),
        owner: node(0),
        value: name("lazy"),
        file: file("string_literal/input.ts"),
        span: span((3, 18), (3, 24)),
        import: true,
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
