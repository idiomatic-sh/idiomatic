// The rows `relations/comment/input.html` must give.

use ryo::languages::contract::{
    CommentExpectedRow, Expected, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[CommentExpectedRow] = &[CommentExpectedRow {
    id: node(16),
    text: name(" note "),
    file: file("comment/input.html"),
    span: span((1, 35), (1, 48)),
    doc: false,
}];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
