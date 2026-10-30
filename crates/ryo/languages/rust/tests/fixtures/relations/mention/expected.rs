// The rows `relations/mention/input.rs` must give.

use ryo::languages::contract::{
    Expected, MentionExpectedRow, expected_rows, file, name, node, span,
};

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(&[
        MentionExpectedRow {
            occurrence: node(24),
            owner: node(1),
            target: name("alpha"),
            file: file("mention/input.rs"),
            span: span((2, 14), (2, 19)),
        },
        MentionExpectedRow {
            occurrence: node(27),
            owner: node(1),
            target: name("beta"),
            file: file("mention/input.rs"),
            span: span((2, 21), (2, 25)),
        },
        MentionExpectedRow {
            occurrence: node(44),
            owner: node(1),
            target: name("gamma"),
            file: file("mention/input.rs"),
            span: span((3, 20), (3, 25)),
        },
    ])
}
