//! A numeric primitive points to its own normalized key representative.

use ryo::languages::contract::{ConstantKeyExpectedRow, Expected, expected_rows, file, node, span};

const ROWS: &[ConstantKeyExpectedRow] = &[ConstantKeyExpectedRow {
    id: node(2),
    key: node(2),
    file: file("constant_key/input.py"),
    span: span((1, 1), (1, 2)),
}];

pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
