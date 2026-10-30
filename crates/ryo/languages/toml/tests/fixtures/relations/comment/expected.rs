// The rows `relations/comment/input.toml` must give.

use ryo::languages::contract::{Expected, CommentExpectedRow, expected_rows, file, name, node, span};

/// The rows, in the order the language gives them.
const ROWS: &[CommentExpectedRow] = &[
    CommentExpectedRow {
        id: node(1),
        text: name(" first"),
        file: file("comment/input.toml"),
        span: span((1, 1), (1, 8)),
        doc: false,
    },
    CommentExpectedRow {
        id: node(11),
        text: name(" trailing"),
        file: file("comment/input.toml"),
        span: span((2, 7), (2, 17)),
        doc: false,
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
