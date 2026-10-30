// The rows `relations/reference/input.rs` must give.

use ryo::languages::contract::{
    Expected, ReferenceExpectedRow, expected_rows, file, name, node, span,
};

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(&[
        ReferenceExpectedRow {
            occurrence: node(15),
            owner: node(1),
            target: name("crate::Value"),
            file: file("reference/input.rs"),
            span: span((1, 15), (1, 27)),
        },
        ReferenceExpectedRow {
            occurrence: node(33),
            owner: node(1),
            target: name("consume"),
            file: file("reference/input.rs"),
            span: span((1, 31), (1, 38)),
        },
        ReferenceExpectedRow {
            occurrence: node(40),
            owner: node(1),
            target: name("value"),
            file: file("reference/input.rs"),
            span: span((1, 39), (1, 44)),
        },
        ReferenceExpectedRow {
            occurrence: node(77),
            owner: node(0),
            target: name("Handle"),
            file: file("reference/input.rs"),
            span: span((4, 16), (4, 22)),
        },
        ReferenceExpectedRow {
            occurrence: node(86),
            owner: node(0),
            target: name("Handle::new"),
            file: file("reference/input.rs"),
            span: span((4, 25), (4, 36)),
        },
        ReferenceExpectedRow {
            occurrence: node(114),
            owner: node(100),
            target: name("u8"),
            file: file("reference/input.rs"),
            span: span((6, 17), (6, 19)),
        },
        ReferenceExpectedRow {
            occurrence: node(127),
            owner: node(100),
            target: name("println"),
            file: file("reference/input.rs"),
            span: span((7, 5), (7, 12)),
        },
        ReferenceExpectedRow {
            occurrence: node(146),
            owner: node(100),
            target: name("value"),
            file: file("reference/input.rs"),
            span: span((7, 20), (7, 25)),
        },
    ])
}
