// The rows `relations/static_call/input.rs` must give.

use ryo::languages::contract::{
    Expected, StaticCallExpectedRow, expected_rows, file, name, node, span,
};

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(&[
        StaticCallExpectedRow {
            occurrence: node(22),
            owner: node(1),
            target: name("stop"),
            file: file("static_call/input.rs"),
            span: span((1, 17), (1, 21)),
        },
        StaticCallExpectedRow {
            occurrence: node(27),
            owner: node(1),
            target: name("super::make"),
            file: file("static_call/input.rs"),
            span: span((1, 22), (1, 39)),
        },
        StaticCallExpectedRow {
            occurrence: node(70),
            owner: node(55),
            target: name("crate::service"),
            file: file("static_call/input.rs"),
            span: span((3, 12), (3, 26)),
        },
    ])
}
