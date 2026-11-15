// The rows `relations/attribute_path/input.rs` must give.

use ryo::languages::contract::{
    AttributePathExpectedRow, Expected, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[AttributePathExpectedRow] = &[
    AttributePathExpectedRow {
        id: node(6),
        name: name("rustfmt::skip"),
        file: file("attribute_path/input.rs"),
        span: span((1, 3), (1, 16)),
    },
    AttributePathExpectedRow {
        id: node(21),
        name: name("deny"),
        file: file("attribute_path/input.rs"),
        span: span((2, 3), (2, 7)),
    },
    AttributePathExpectedRow {
        id: node(27),
        name: name("clippy::len_zero"),
        file: file("attribute_path/input.rs"),
        span: span((2, 8), (2, 35)),
    },
    AttributePathExpectedRow {
        id: node(36),
        name: name("dead_code"),
        file: file("attribute_path/input.rs"),
        span: span((2, 37), (2, 46)),
    },
    AttributePathExpectedRow {
        id: node(51),
        name: name("rustfmt::skip"),
        file: file("attribute_path/input.rs"),
        span: span((3, 18), (3, 31)),
    },
    AttributePathExpectedRow {
        id: node(62),
        name: name("::a::r#b"),
        file: file("attribute_path/input.rs"),
        span: span((3, 32), (3, 40)),
    },
    AttributePathExpectedRow {
        id: node(70),
        name: name("c"),
        file: file("attribute_path/input.rs"),
        span: span((3, 42), (3, 43)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
