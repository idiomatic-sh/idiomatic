// Independent source expression and token identities for one ordinary string.
use ryo::languages::contract::{
    Expected, RawFormatSegmentExpectedRow, expected_rows, file, name, node, number, span,
};
const ROWS: &[RawFormatSegmentExpectedRow] = &[RawFormatSegmentExpectedRow {
    id: node(2),
    segment: node(3),
    segment_ordinal: number(0),
    dialect: name("percent-string"),
    outcome: name("parsed"),
    error_category: name("none"),
    error_character: name(""),
    error_index: number(0),
    positional_consumption: number(1),
    mapping_present: false,
    dynamic_width: false,
    dynamic_precision: false,
    file: file("raw_format_segment/input.py"),
    span: span((1, 1), (1, 5)),
}];
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
