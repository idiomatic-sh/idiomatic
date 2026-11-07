// Assignment binds x before its next-line read; the integer is not a name event.

use ryo::languages::contract::{
    BindingEventExpectedRow, Expected, expected_rows, file, name, node, number, span,
};

/// Independently specified complete rows for this input.
pub(super) fn expected() -> Expected {
    expected_rows(&[
        BindingEventExpectedRow {
            occurrence: node(2),
            name: name("x"),
            scope: node(0),
            phase: node(0),
            kind: name("bind"),
            ordinal: number(1),
            file: file("binding_event/input.py"),
            span: span((1, 1), (1, 2)),
        },
        BindingEventExpectedRow {
            occurrence: node(11),
            name: name("x"),
            scope: node(0),
            phase: node(0),
            kind: name("read"),
            ordinal: number(2),
            file: file("binding_event/input.py"),
            span: span((2, 1), (2, 2)),
        },
    ])
}
