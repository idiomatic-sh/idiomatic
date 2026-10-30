// The rows `relations/static_import/input.go` must give.

use ryo::languages::contract::{
    Expected, StaticImportExpectedRow, expected_rows, file, name, node, span, unknown,
};

/// The rows, in the order the language gives them.
const ROWS: &[StaticImportExpectedRow] = &[
    StaticImportExpectedRow {
        occurrence: node(11),
        owner: node(0),
        binding: unknown("declared-by-package"),
        target: name("fmt"),
        file: file("static_import/input.go"),
        span: span((4, 2), (4, 7)),
    },
    StaticImportExpectedRow {
        occurrence: node(15),
        owner: node(0),
        binding: name("str"),
        target: name("strings"),
        file: file("static_import/input.go"),
        span: span((5, 2), (5, 15)),
    },
    StaticImportExpectedRow {
        occurrence: node(22),
        owner: node(0),
        binding: name("_"),
        target: name("embed"),
        file: file("static_import/input.go"),
        span: span((6, 2), (6, 11)),
    },
    StaticImportExpectedRow {
        occurrence: node(29),
        owner: node(0),
        binding: unknown("dot-import"),
        target: name("math"),
        file: file("static_import/input.go"),
        span: span((7, 2), (7, 10)),
    },
    StaticImportExpectedRow {
        occurrence: node(41),
        owner: node(0),
        binding: unknown("declared-by-package"),
        target: name("os"),
        file: file("static_import/input.go"),
        span: span((10, 8), (10, 12)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
