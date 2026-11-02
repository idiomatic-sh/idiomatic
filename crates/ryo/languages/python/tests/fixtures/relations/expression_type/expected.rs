// An integer expression has a known atomic type; no declaration is required.

use ryo::languages::contract::{
    Expected, ExpressionTypeExpectedRow, expected_rows, file, name, node, span,
};

/// Independently specified complete rows for this input.
pub(super) fn expected() -> Expected {
    expected_rows(&[ExpressionTypeExpectedRow {
        id: node(2),
        resolution: name("atom"),
        type_: name("integer"),
        file: file("expression_type/input.py"),
        span: span((1, 1), (1, 2)),
    }])
}
