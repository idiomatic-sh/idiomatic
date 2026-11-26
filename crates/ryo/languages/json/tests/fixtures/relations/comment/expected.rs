// The rows `relations/comment/input.jsonc` must give.

use ryo::languages::contract::{
    CommentExpectedRow, Expected, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[CommentExpectedRow] = &[
    CommentExpectedRow {
        id: node(1),
        text: name(" leading"),
        file: file("comment/input.jsonc"),
        span: span((1, 1), (1, 11)),
        doc: false,
    },
    CommentExpectedRow {
        id: node(6),
        text: name(" block "),
        file: file("comment/input.jsonc"),
        span: span((3, 3), (3, 14)),
        doc: false,
    },
    CommentExpectedRow {
        id: node(15),
        text: name(" trailing"),
        file: file("comment/input.jsonc"),
        span: span((3, 23), (3, 34)),
        doc: false,
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
