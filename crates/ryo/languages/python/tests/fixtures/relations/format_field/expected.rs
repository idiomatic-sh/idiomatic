// One format field points back to the literal and preserves its ordinal.

use ryo::languages::contract::{
    Expected, FormatFieldExpectedRow, expected_rows, file, name, node, number, span,
};

/// One independently specified positional conversion.
const ROWS: &[FormatFieldExpectedRow] = &[FormatFieldExpectedRow {
    id: node(2),
    ordinal: number(0),
    argument_kind: name("positional"),
    mapping_key: name(""),
    conversion: name("s"),
    width_mode: name("none"),
    precision_mode: name("none"),
    file: file("format_field/input.py"),
    span: span((1, 1), (1, 5)),
}];

/// The exact relation rows expected from the source fixture.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
