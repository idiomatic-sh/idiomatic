// The rows `relations/comment/input.css` must give.

use ryo::languages::contract::{Expected, CommentExpectedRow, expected_rows, file, name, node, span};

/// The rows, in the order the language gives them.
const ROWS: &[CommentExpectedRow] = &[
    CommentExpectedRow {
        id: node(1),
        text: name(" the card "),
        file: file("comment/input.css"),
        span: span((1, 1), (1, 15)),
        doc: false,
    },
    CommentExpectedRow {
        id: node(9),
        text: name(" inner "),
        file: file("comment/input.css"),
        span: span((2, 9), (2, 20)),
        doc: false,
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
