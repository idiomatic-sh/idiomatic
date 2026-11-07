// The rows `relations/comment/input.ts` must give.

use ryo::languages::contract::{
    CommentExpectedRow, Expected, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[CommentExpectedRow] = &[
    CommentExpectedRow {
        id: node(1),
        text: name(" hello"),
        file: file("comment/input.ts"),
        span: span((1, 1), (1, 9)),
        doc: false,
    },
    CommentExpectedRow {
        id: node(3),
        text: name("* Greets. "),
        file: file("comment/input.ts"),
        span: span((2, 1), (2, 15)),
        doc: true,
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
