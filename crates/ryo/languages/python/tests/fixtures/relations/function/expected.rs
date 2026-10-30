// The rows `relations/function/input.py` must give.

use ryo::languages::contract::{
    Expected, FunctionExpectedRow, expected_rows, file, name, node, span,
};

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(&[
        FunctionExpectedRow {
            id: node(1),
            name: name("top"),
            file: file("function/input.py"),
            span: span((1, 1), (4, 18)),
        },
        FunctionExpectedRow {
            id: node(28),
            name: name("nested"),
            file: file("function/input.py"),
            span: span((2, 5), (3, 13)),
        },
        FunctionExpectedRow {
            id: node(61),
            name: name("fetch"),
            file: file("function/input.py"),
            span: span((8, 5), (9, 25)),
        },
        FunctionExpectedRow {
            id: node(96),
            name: name("build"),
            file: file("function/input.py"),
            span: span((11, 5), (13, 25)),
        },
    ])
}
