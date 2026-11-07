// The rows `relations/comment/input.md` must give.

use ryo::languages::contract::{
    CommentExpectedRow, Expected, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[CommentExpectedRow] = &[
    CommentExpectedRow {
        id: node(1),
        text: name(" A comment of its own. "),
        file: file("comment/input.md"),
        span: span((1, 1), (2, 1)),
        doc: false,
    },
    CommentExpectedRow {
        id: node(6),
        text: name(" an inline comment "),
        file: file("comment/input.md"),
        span: span((3, 11), (3, 37)),
        doc: false,
    },
    CommentExpectedRow {
        id: node(10),
        text: name("\nA comment\nover lines.\n"),
        file: file("comment/input.md"),
        span: span((5, 1), (9, 1)),
        doc: false,
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
