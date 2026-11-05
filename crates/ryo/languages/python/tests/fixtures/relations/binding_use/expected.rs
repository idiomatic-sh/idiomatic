// A read refers to the actual preceding assignment target, not the value literal.

use ryo::languages::contract::{
    BindingUseExpectedRow, Expected, expected_rows, file, name, node, span,
};

/// Independently specified complete rows for this input.
pub(super) fn expected() -> Expected {
    expected_rows(&[BindingUseExpectedRow {
        reference: node(11),
        binding: node(2),
        scope: node(0),
        phase: node(0),
        context: name("value"),
        certainty: name("unique"),
        file: file("binding_use/input.py"),
        span: span((2, 1), (2, 2)),
    }])
}
