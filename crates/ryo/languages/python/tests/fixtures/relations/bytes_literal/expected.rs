// Decoded octets use hexadecimal, not their original spelling.
use ryo::languages::contract::{
    BytesLiteralExpectedRow, Expected, expected_rows, file, name, node, span,
};
/// Independently specified complete decoded rows.
pub(super) fn expected() -> Expected {
    expected_rows(&[BytesLiteralExpectedRow {
        occurrence: node(2),
        owner: node(0),
        hex_value: name("6162"),
        file: file("bytes_literal/input.py"),
        span: span((1, 1), (1, 6)),
    }])
}
