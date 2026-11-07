// The rows `relations/annotates/input.rs` must give.

use ryo::languages::contract::{
    AnnotatesExpectedRow, Expected, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[AnnotatesExpectedRow] = &[
    AnnotatesExpectedRow {
        occurrence: node(22),
        owner: node(18),
        annotation: name("crate::Value"),
        file: file("annotates/input.rs"),
        span: span((1, 23), (1, 35)),
    },
    AnnotatesExpectedRow {
        occurrence: node(54),
        owner: node(50),
        annotation: name("u8"),
        file: file("annotates/input.rs"),
        span: span((3, 15), (3, 17)),
    },
    AnnotatesExpectedRow {
        occurrence: node(64),
        owner: node(41),
        annotation: name("bool"),
        file: file("annotates/input.rs"),
        span: span((3, 22), (3, 26)),
    },
    AnnotatesExpectedRow {
        occurrence: node(114),
        owner: node(107),
        annotation: name("u8"),
        file: file("annotates/input.rs"),
        span: span((5, 14), (5, 16)),
    },
    AnnotatesExpectedRow {
        occurrence: node(135),
        owner: node(126),
        annotation: name("crate::Count"),
        file: file("annotates/input.rs"),
        span: span((6, 19), (6, 31)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
