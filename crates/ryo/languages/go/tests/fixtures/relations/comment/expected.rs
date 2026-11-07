// The rows `relations/comment/input.go` must give.

use ryo::languages::contract::{
    CommentExpectedRow, Expected, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[CommentExpectedRow] = &[
    CommentExpectedRow {
        id: node(1),
        text: name(" Package p is the fixture of the comment relation."),
        file: file("comment/input.go"),
        span: span((1, 1), (1, 53)),
        doc: true,
    },
    CommentExpectedRow {
        id: node(8),
        text: name(" a block\n   comment "),
        file: file("comment/input.go"),
        span: span((4, 1), (5, 14)),
        doc: true,
    },
    CommentExpectedRow {
        id: node(22),
        text: name(" trailing"),
        file: file("comment/input.go"),
        span: span((6, 11), (6, 22)),
        doc: false,
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
