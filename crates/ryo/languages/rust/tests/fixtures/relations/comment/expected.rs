// The rows `relations/comment/input.rs` must give.

use ryo::languages::contract::{
    CommentExpectedRow, Expected, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[CommentExpectedRow] = &[
    CommentExpectedRow {
        id: node(1),
        text: name(" Crate docs."),
        file: file("comment/input.rs"),
        span: span((1, 1), (1, 16)),
        doc: true,
    },
    CommentExpectedRow {
        id: node(4),
        text: name(" first"),
        file: file("comment/input.rs"),
        span: span((2, 1), (2, 9)),
        doc: false,
    },
    CommentExpectedRow {
        id: node(6),
        text: name(" doc"),
        file: file("comment/input.rs"),
        span: span((3, 1), (3, 8)),
        doc: true,
    },
    CommentExpectedRow {
        id: node(22),
        text: name(" block "),
        file: file("comment/input.rs"),
        span: span((4, 13), (4, 24)),
        doc: false,
    },
    CommentExpectedRow {
        id: node(24),
        text: name(" Outer block doc. "),
        file: file("comment/input.rs"),
        span: span((5, 1), (5, 24)),
        doc: true,
    },
    CommentExpectedRow {
        id: node(39),
        text: name("// plain"),
        file: file("comment/input.rs"),
        span: span((7, 1), (7, 11)),
        doc: false,
    },
    CommentExpectedRow {
        id: node(44),
        text: name(" kept"),
        file: file("comment/input.rs"),
        span: span((10, 1), (10, 8)),
        doc: false,
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
