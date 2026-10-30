// The rows `relations/static_call/input.py` must give.

use ryo::languages::contract::{
    Expected, StaticCallExpectedRow, expected_rows, file, name, node, span, unknown,
};

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(&[
        StaticCallExpectedRow {
            occurrence: node(10),
            owner: node(0),
            target: name("print"),
            file: file("static_call/input.py"),
            span: span((3, 1), (3, 6)),
        },
        StaticCallExpectedRow {
            occurrence: node(21),
            owner: node(0),
            target: name("os.path.join"),
            file: file("static_call/input.py"),
            span: span((4, 1), (4, 13)),
        },
        StaticCallExpectedRow {
            occurrence: node(45),
            owner: node(0),
            target: name("logger.info"),
            file: file("static_call/input.py"),
            span: span((5, 1), (5, 12)),
        },
        StaticCallExpectedRow {
            occurrence: node(72),
            owner: node(0),
            target: unknown("dynamic-callee"),
            file: file("static_call/input.py"),
            span: span((6, 1), (6, 15)),
        },
        StaticCallExpectedRow {
            occurrence: node(87),
            owner: node(0),
            target: unknown("dynamic-callee"),
            file: file("static_call/input.py"),
            span: span((7, 1), (7, 7)),
        },
        StaticCallExpectedRow {
            occurrence: node(88),
            owner: node(0),
            target: name("make"),
            file: file("static_call/input.py"),
            span: span((7, 1), (7, 5)),
        },
        StaticCallExpectedRow {
            occurrence: node(99),
            owner: node(0),
            target: unknown("dynamic-callee"),
            file: file("static_call/input.py"),
            span: span((8, 1), (8, 9)),
        },
        StaticCallExpectedRow {
            occurrence: node(114),
            owner: node(0),
            target: unknown("dynamic-callee"),
            file: file("static_call/input.py"),
            span: span((9, 1), (9, 14)),
        },
        StaticCallExpectedRow {
            occurrence: node(116),
            owner: node(0),
            target: name("super"),
            file: file("static_call/input.py"),
            span: span((9, 1), (9, 6)),
        },
        StaticCallExpectedRow {
            occurrence: node(130),
            owner: node(0),
            target: unknown("dynamic-callee"),
            file: file("static_call/input.py"),
            span: span((10, 2), (10, 11)),
        },
    ])
}
