// The rows `relations/function/input.rs` must give.

use ryo::languages::contract::{
    Expected, FunctionExpectedRow, expected_rows, file, name, node, span,
};

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(&[
        FunctionExpectedRow {
            id: node(1),
            name: name("outer"),
            file: file("function/input.rs"),
            span: span((1, 1), (1, 29)),
        },
        FunctionExpectedRow {
            id: node(14),
            name: name("inner"),
            file: file("function/input.rs"),
            span: span((1, 14), (1, 27)),
        },
        FunctionExpectedRow {
            id: node(30),
            name: name("run"),
            file: file("function/input.rs"),
            span: span((3, 1), (3, 12)),
        },
        FunctionExpectedRow {
            id: node(44),
            name: name("run"),
            file: file("function/input.rs"),
            span: span((5, 1), (5, 12)),
        },
    ])
}
