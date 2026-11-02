// Keep segment/local/global field order; the malformed middle token contributes no valid-prefix fields.

use ryo::languages::contract::{
    Expected, RawFormatFieldExpectedRow, expected_rows, file, name, node, number, span,
};

/// Independently specified full rows, including negative/default values and real source positions.
const ROWS: &[RawFormatFieldExpectedRow] = &[
    RawFormatFieldExpectedRow {
        id: node(2),
        segment: node(4),
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
    },
    RawFormatFieldExpectedRow {
        id: node(7),
        segment: node(15),
        segment_ordinal: number(2),
        ordinal: number(0),
        segment_field_ordinal: number(0),
        argument_kind: name("mapping"),
        mapping_key: name("x"),
        conversion: name("d"),
        width_mode: name("none"),
        precision_mode: name("none"),
        file: file("raw_format_field/input.py"),
        span: span((2, 9), (2, 28)),
    },
    RawFormatFieldExpectedRow {
        id: node(7),
        segment: node(15),
        segment_ordinal: number(2),
        ordinal: number(1),
        segment_field_ordinal: number(1),
        argument_kind: name("mapping"),
        mapping_key: name("x"),
        conversion: name("s"),
        width_mode: name("none"),
        precision_mode: name("none"),
        file: file("raw_format_field/input.py"),
        span: span((2, 9), (2, 28)),
    },
    RawFormatFieldExpectedRow {
        id: node(7),
        segment: node(15),
        segment_ordinal: number(2),
        ordinal: number(2),
        segment_field_ordinal: number(2),
        argument_kind: name("positional"),
        mapping_key: name(""),
        conversion: name("d"),
        width_mode: name("star"),
        precision_mode: name("star"),
        file: file("raw_format_field/input.py"),
        span: span((2, 9), (2, 28)),
    },
    RawFormatFieldExpectedRow {
        id: node(18),
        segment: node(20),
        segment_ordinal: number(0),
        ordinal: number(0),
        segment_field_ordinal: number(0),
        argument_kind: name("positional"),
        mapping_key: name(""),
        conversion: name("s"),
        width_mode: name("none"),
        precision_mode: name("none"),
        file: file("raw_format_field/input.py"),
        span: span((3, 1), (3, 5)),
    },
    RawFormatFieldExpectedRow {
        id: node(18),
        segment: node(26),
        segment_ordinal: number(2),
        ordinal: number(1),
        segment_field_ordinal: number(0),
        argument_kind: name("positional"),
        mapping_key: name(""),
        conversion: name("x"),
        width_mode: name("none"),
        precision_mode: name("none"),
        file: file("raw_format_field/input.py"),
        span: span((3, 13), (3, 17)),
    },
];

/// Exact expected facts for the coverage fixture; no facet exemptions are declared.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
