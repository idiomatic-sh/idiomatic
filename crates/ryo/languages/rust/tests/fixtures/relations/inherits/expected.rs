// The rows `relations/inherits/input.rs` must give.

use ryo::languages::contract::{
    Expected, InheritsExpectedRow, expected_rows, file, name, node, span,
};

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(&[
        InheritsExpectedRow {
            occurrence: node(21),
            declaration: node(11),
            target: name("Parent"),
            file: file("inherits/input.rs"),
            span: span((1, 31), (1, 37)),
        },
        InheritsExpectedRow {
            occurrence: node(42),
            declaration: node(32),
            target: name("Parent"),
            file: file("inherits/input.rs"),
            span: span((1, 63), (1, 69)),
        },
        InheritsExpectedRow {
            occurrence: node(79),
            declaration: node(70),
            target: name("Base"),
            file: file("inherits/input.rs"),
            span: span((4, 17), (4, 21)),
        },
        InheritsExpectedRow {
            occurrence: node(88),
            declaration: node(70),
            target: name("Clone"),
            file: file("inherits/input.rs"),
            span: span((4, 24), (4, 29)),
        },
        InheritsExpectedRow {
            occurrence: node(101),
            declaration: node(98),
            target: name("Contract"),
            file: file("inherits/input.rs"),
            span: span((5, 6), (5, 14)),
        },
    ])
}
