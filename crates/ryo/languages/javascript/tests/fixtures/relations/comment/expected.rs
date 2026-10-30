// The rows `relations/comment/input.js` must give.

use ryo::languages::contract::{
    CommentExpectedRow, Expected, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[CommentExpectedRow] = &[
    CommentExpectedRow {
        id: node(1),
        text: name(" A module exercises bindings, calls and decoded strings."),
        file: file("comment/input.js"),
        span: span((1, 1), (1, 59)),
        doc: false,
    },
    CommentExpectedRow {
        id: node(28),
        text: name("* Greets the name. "),
        file: file("comment/input.js"),
        span: span((3, 1), (3, 24)),
        doc: true,
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
