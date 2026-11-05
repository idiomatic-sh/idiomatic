// An adjacent decoded literal belongs to its f-string; the separate plain string has no owner.

use ryo::languages::contract::{
    Expected, InterpolatedStringLiteralExpectedRow, expected_rows, file, name, node, span,
};

/// Independently specified complete rows for this input.
pub(super) fn expected() -> Expected {
    expected_rows(&[InterpolatedStringLiteralExpectedRow {
        id: node(3),
        owner: node(2),
        value: name("text"),
        file: file("interpolated_string_literal/input.py"),
        span: span((1, 1), (1, 7)),
    }])
}
