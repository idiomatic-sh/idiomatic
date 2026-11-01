//! A decoded primitive string retains its original Unicode source span.

use ryo::languages::contract::{
    Expected, LiteralPropertiesExpectedRow, expected_rows, file, name, node, span, want_flag,
};

const ROWS: &[LiteralPropertiesExpectedRow] = &[LiteralPropertiesExpectedRow {
    id: node(2),
    kind: name("string"),
    zero: want_flag(false),
    empty: want_flag(false),
    boolean: want_flag(false),
    file: file("literal_properties/input.py"),
    span: span((1, 1), (1, 5)),
}];

pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
