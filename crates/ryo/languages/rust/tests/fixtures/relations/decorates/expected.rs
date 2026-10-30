// The rows `relations/decorates/input.rs` must give.

use ryo::languages::contract::{
    DecoratesExpectedRow, Expected, expected_rows, file, name, node, number, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[DecoratesExpectedRow] = &[
    DecoratesExpectedRow {
        occurrence: node(2),
        declaration: node(1),
        ordinal: number(0),
        target: name("inline"),
        arguments: name(""),
        file: file("decorates/input.rs"),
        span: span((1, 1), (1, 10)),
    },
    DecoratesExpectedRow {
        occurrence: node(26),
        declaration: node(25),
        ordinal: number(0),
        target: name("derive"),
        arguments: name("Debug, Clone"),
        file: file("decorates/input.rs"),
        span: span((4, 1), (4, 24)),
    },
    DecoratesExpectedRow {
        occurrence: node(43),
        declaration: node(25),
        ordinal: number(1),
        target: name("serde"),
        arguments: name("rename = \"x\""),
        file: file("decorates/input.rs"),
        span: span((5, 1), (5, 23)),
    },
    DecoratesExpectedRow {
        occurrence: node(70),
        declaration: node(69),
        ordinal: number(0),
        target: name("allow"),
        arguments: name("dead_code"),
        file: file("decorates/input.rs"),
        span: span((7, 5), (7, 24)),
    },
    DecoratesExpectedRow {
        occurrence: node(98),
        declaration: node(97),
        ordinal: number(0),
        target: name("path"),
        arguments: name("\"inner.rs\""),
        file: file("decorates/input.rs"),
        span: span((11, 1), (11, 21)),
    },
    DecoratesExpectedRow {
        occurrence: node(120),
        declaration: node(119),
        ordinal: number(0),
        target: name("cfg"),
        arguments: name("test"),
        file: file("decorates/input.rs"),
        span: span((14, 1), (14, 13)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
