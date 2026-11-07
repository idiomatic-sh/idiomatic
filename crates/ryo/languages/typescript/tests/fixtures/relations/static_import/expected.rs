// The rows `relations/static_import/input.ts` must give.

use ryo::languages::contract::{
    Expected, StaticImportExpectedRow, expected_rows, file, name, node, span, unknown,
};

/// The rows, in the order the language gives them.
const ROWS: &[StaticImportExpectedRow] = &[
    StaticImportExpectedRow {
        occurrence: node(1),
        owner: node(0),
        binding: name("b"),
        target: name("pkg"),
        file: file("static_import/input.ts"),
        span: span((1, 1), (1, 28)),
    },
    StaticImportExpectedRow {
        occurrence: node(30),
        owner: node(0),
        binding: unknown("dynamic-or-anonymous"),
        target: unknown("dynamic-or-anonymous"),
        file: file("static_import/input.ts"),
        span: span((2, 15), (2, 33)),
    },
    StaticImportExpectedRow {
        occurrence: node(39),
        owner: node(0),
        binding: unknown("dynamic-or-anonymous"),
        target: name("./literal.js"),
        file: file("static_import/input.ts"),
        span: span((3, 1), (3, 23)),
    },
    StaticImportExpectedRow {
        occurrence: node(56),
        owner: node(0),
        binding: unknown("dynamic-or-anonymous"),
        target: name("./b.js"),
        file: file("static_import/input.ts"),
        span: span((4, 13), (4, 30)),
    },
    StaticImportExpectedRow {
        occurrence: node(66),
        owner: node(0),
        binding: unknown("dynamic-or-anonymous"),
        target: unknown("dynamic-or-anonymous"),
        file: file("static_import/input.ts"),
        span: span((5, 1), (5, 18)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
