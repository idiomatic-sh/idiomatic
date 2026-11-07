// The rows `relations/static_import/input.js` must give.

use ryo::languages::contract::{
    Expected, StaticImportExpectedRow, expected_rows, file, name, node, span, unknown,
};

/// The rows, in the order the language gives them.
const ROWS: &[StaticImportExpectedRow] = &[
    StaticImportExpectedRow {
        occurrence: node(3),
        owner: node(0),
        binding: name("load"),
        target: name("module"),
        file: file("static_import/input.js"),
        span: span((2, 1), (2, 43)),
    },
    StaticImportExpectedRow {
        occurrence: node(3),
        owner: node(0),
        binding: name("tool"),
        target: name("module"),
        file: file("static_import/input.js"),
        span: span((2, 1), (2, 43)),
    },
    StaticImportExpectedRow {
        occurrence: node(284),
        owner: node(0),
        binding: name("reader"),
        target: name("escaped"),
        file: file("static_import/input.js"),
        span: span((16, 1), (16, 45)),
    },
    StaticImportExpectedRow {
        occurrence: node(313),
        owner: node(0),
        binding: unknown("dynamic-or-anonymous"),
        target: unknown("dynamic-or-anonymous"),
        file: file("static_import/input.js"),
        span: span((17, 15), (17, 33)),
    },
    StaticImportExpectedRow {
        occurrence: node(322),
        owner: node(0),
        binding: unknown("dynamic-or-anonymous"),
        target: name("./literal.js"),
        file: file("static_import/input.js"),
        span: span((18, 1), (18, 23)),
    },
    StaticImportExpectedRow {
        occurrence: node(339),
        owner: node(0),
        binding: unknown("dynamic-or-anonymous"),
        target: name("./b.js"),
        file: file("static_import/input.js"),
        span: span((19, 13), (19, 30)),
    },
    StaticImportExpectedRow {
        occurrence: node(349),
        owner: node(0),
        binding: unknown("dynamic-or-anonymous"),
        target: unknown("dynamic-or-anonymous"),
        file: file("static_import/input.js"),
        span: span((20, 1), (20, 18)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
