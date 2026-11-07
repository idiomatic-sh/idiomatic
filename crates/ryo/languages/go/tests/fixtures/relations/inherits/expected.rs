// The rows `relations/inherits/input.go` must give.

use ryo::languages::contract::{
    Expected, InheritsExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[InheritsExpectedRow] = &[
    InheritsExpectedRow {
        occurrence: node(20),
        declaration: node(9),
        target: name("Base"),
        file: file("inherits/input.go"),
        span: span((4, 2), (4, 6)),
    },
    InheritsExpectedRow {
        occurrence: node(24),
        declaration: node(9),
        target: name("Logger"),
        file: file("inherits/input.go"),
        span: span((5, 2), (5, 9)),
    },
    InheritsExpectedRow {
        occurrence: node(30),
        declaration: node(9),
        target: name("sync.Mutex"),
        file: file("inherits/input.go"),
        span: span((6, 2), (6, 12)),
    },
    InheritsExpectedRow {
        occurrence: node(38),
        declaration: node(9),
        target: name("Cache"),
        file: file("inherits/input.go"),
        span: span((7, 2), (7, 15)),
    },
    InheritsExpectedRow {
        occurrence: node(69),
        declaration: node(58),
        target: name("io.Reader"),
        file: file("inherits/input.go"),
        span: span((12, 2), (12, 11)),
    },
    InheritsExpectedRow {
        occurrence: node(119),
        declaration: node(94),
        target: name("comparable"),
        file: file("inherits/input.go"),
        span: span((18, 2), (18, 12)),
    },
    InheritsExpectedRow {
        occurrence: node(138),
        declaration: node(131),
        target: name("Embedded"),
        file: file("inherits/input.go"),
        span: span((22, 2), (22, 10)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
