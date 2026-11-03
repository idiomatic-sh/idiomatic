// A standalone literal exposes format syntax without a method call.
use ryo::languages::contract::{
    Expected, FormatSpecificationExpectedRow, expected_rows, file, name, node, number, span,
};
/// Independently specified complete decoded rows.
pub(super) fn expected() -> Expected {
    expected_rows(&[FormatSpecificationExpectedRow {
        id: node(2),
        ordinal: number(0),
        parent_ordinal: number(-1),
        origin: node(2),
        source_kind: name("brace-string"),
        value: name("q"),
        outcome: name("invalid-type"),
        invalid_type: name("q"),
        selector: name(""),
        conversion: name(""),
        file: file("format_specification/input.py"),
        span: span((1, 1), (1, 7)),
    }])
}
