// A builtin read has availability but no invented declaration.

use ryo::languages::contract::{
    BindingReadExpectedRow, Expected, expected_rows, file, name, node, number, span,
};

/// Independently specified complete rows for this input.
pub(super) fn expected() -> Expected {
    expected_rows(&[
        BindingReadExpectedRow {
            reference: node(2),
            name: name("len"),
            scope: node(0),
            phase: node(0),
            context: name("value"),
            state: name("builtin"),
            since_major: number(3),
            since_minor: number(0),
            file: file("binding_read/input.py"),
            span: span((1, 1), (1, 4)),
        },
        BindingReadExpectedRow {
            reference: node(2),
            name: name("aiter"),
            scope: node(0),
            phase: node(0),
            context: name("value"),
            state: name("builtin"),
            since_major: number(3),
            since_minor: number(10),
            file: file("binding_read/aiter.py"),
            span: span((1, 1), (1, 6)),
        },
    ])
}
