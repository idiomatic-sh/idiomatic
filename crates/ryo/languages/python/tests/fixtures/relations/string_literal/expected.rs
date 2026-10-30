// The rows `relations/string_literal/input.py` must give.

use ryo::languages::contract::{
    Expected, StringLiteralExpectedRow, expected_rows, file, name, node, span, unknown,
};

/// The rows, in the order the language gives them.
const ROWS: &[StringLiteralExpectedRow] = &[
    StringLiteralExpectedRow {
        occurrence: node(2),
        owner: node(0),
        value: name("The module's docstring."),
        file: file("string_literal/input.py"),
        span: span((1, 1), (1, 30)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(15),
        owner: node(0),
        value: name("export"),
        file: file("string_literal/input.py"),
        span: span((3, 12), (3, 20)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(20),
        owner: node(0),
        value: name("other"),
        file: file("string_literal/input.py"),
        span: span((3, 22), (3, 29)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(31),
        owner: node(0),
        value: name("line\njoined"),
        file: file("string_literal/input.py"),
        span: span((4, 9), (4, 26)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(44),
        owner: node(0),
        value: name("\\d+"),
        file: file("string_literal/input.py"),
        span: span((5, 7), (5, 13)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(54),
        owner: node(0),
        value: name("no {braces} here"),
        file: file("string_literal/input.py"),
        span: span((6, 10), (6, 31)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(67),
        owner: node(0),
        value: unknown("interpolated-string"),
        file: file("string_literal/input.py"),
        span: span((7, 10), (7, 42)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(74),
        owner: node(0),
        value: unknown("interpolated-string"),
        file: file("string_literal/input.py"),
        span: span((7, 19), (7, 35)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(96),
        owner: node(0),
        value: unknown("interpolated-string"),
        file: file("string_literal/input.py"),
        span: span((8, 12), (8, 27)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(124),
        owner: node(0),
        value: name("\nmany lines\n"),
        file: file("string_literal/input.py"),
        span: span((10, 9), (12, 4)),
        import: false,
    },
    StringLiteralExpectedRow {
        occurrence: node(143),
        owner: node(0),
        value: name("plugins.extra"),
        file: file("string_literal/input.py"),
        span: span((13, 34), (13, 49)),
        import: true,
    },
    StringLiteralExpectedRow {
        occurrence: node(159),
        owner: node(0),
        value: name("os"),
        file: file("string_literal/input.py"),
        span: span((14, 21), (14, 25)),
        import: true,
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
