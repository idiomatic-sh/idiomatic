// One conversion, carrying its expression and existing token independently.
use ryo::languages::contract::{
    Expected, RawFormatFieldExpectedRow, expected_rows, file, name, node, number, span,
};
const ROWS: &[RawFormatFieldExpectedRow] = &[RawFormatFieldExpectedRow {
    id: node(2),
    segment: node(3),
    segment_ordinal: number(0),
    ordinal: number(0),
    segment_field_ordinal: number(0),
    argument_kind: name("positional"),
    mapping_key: name(""),
    conversion: name("s"),
    width_mode: name("none"),
    precision_mode: name("none"),
    file: file("raw_format_field/input.py"),
    span: span((1, 1), (1, 5)),
}];
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
