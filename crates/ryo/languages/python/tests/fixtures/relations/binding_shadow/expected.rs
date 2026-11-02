// A second module assignment replaces the first declaration in the same namespace.

use ryo::languages::contract::{
    BindingShadowExpectedRow, Expected, expected_rows, file, name, node,
};

/// Independently specified complete rows for this input.
pub(super) fn expected() -> Expected {
    expected_rows(&[BindingShadowExpectedRow {
        binding: node(11),
        previous: node(2),
        scope: node(0),
        previous_scope: node(0),
        phase: node(0),
        relationship: name("replacement"),
        file: file("binding_shadow/input.py"),
    }])
}
