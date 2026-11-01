// A decoded primitive string retains its original Unicode source span.

use ryo::languages::contract::{
    Expected, LiteralPropertiesExpectedRow, expected_rows, file, name, node, span,
};

/// Expected parser-derived metadata for the decoded Unicode string.
const ROWS: &[LiteralPropertiesExpectedRow] = &[LiteralPropertiesExpectedRow {
    id: node(2),
    kind: name("string"),
    zero: false,
    empty: false,
    boolean: false,
    file: file("literal_properties/input.py"),
    span: span((1, 1), (1, 5)),
}];

/// Build the contract expectation for the literal-property fixture.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
