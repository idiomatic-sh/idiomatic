// The rows `relations/static_import/input.css` must give.

use ryo::languages::contract::{Expected, StaticImportExpectedRow, expected_rows, file, name, node, span, unknown};

/// The rows, in the order the language gives them.
const ROWS: &[StaticImportExpectedRow] = &[
    StaticImportExpectedRow {
        occurrence: node(1),
        owner: node(0),
        binding: unknown("imports-every-rule"),
        target: name("static_import/base.css"),
        file: file("static_import/input.css"),
        span: span((1, 1), (1, 19)),
    },
    StaticImportExpectedRow {
        occurrence: node(7),
        owner: node(0),
        binding: unknown("imports-every-rule"),
        target: name("static_import/theme.css"),
        file: file("static_import/input.css"),
        span: span((2, 1), (2, 30)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
