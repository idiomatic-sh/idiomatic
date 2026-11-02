// Empty and escaped tokens retain separate identities; later mapping/star and malformed tokens retain their summaries.

use ryo::languages::contract::{
    Expected, RawFormatSegmentExpectedRow, expected_rows, file, name, node, number, span,
};

/// Independently specified full rows, including negative/default values and real source positions.
const ROWS: &[RawFormatSegmentExpectedRow] = &[
    RawFormatSegmentExpectedRow {
        id: node(2),
        segment: node(4),
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
    },
    RawFormatSegmentExpectedRow {
        id: node(7),
        segment: node(9),
        segment_ordinal: number(0),
        dialect: name("percent-string"),
        outcome: name("parsed"),
        error_category: name("none"),
        error_character: name(""),
        error_index: number(0),
        positional_consumption: number(0),
        mapping_present: false,
        dynamic_width: false,
        dynamic_precision: false,
        file: file("raw_format_segment/input.py"),
        span: span((2, 1), (2, 3)),
    },
    RawFormatSegmentExpectedRow {
        id: node(7),
        segment: node(12),
        segment_ordinal: number(1),
        dialect: name("percent-string"),
        outcome: name("parsed"),
        error_category: name("none"),
        error_character: name(""),
        error_index: number(0),
        positional_consumption: number(0),
        mapping_present: false,
        dynamic_width: false,
        dynamic_precision: false,
        file: file("raw_format_segment/input.py"),
        span: span((2, 4), (2, 8)),
    },
    RawFormatSegmentExpectedRow {
        id: node(7),
        segment: node(15),
        segment_ordinal: number(2),
        dialect: name("percent-string"),
        outcome: name("parsed"),
        error_category: name("none"),
        error_character: name(""),
        error_index: number(0),
        positional_consumption: number(3),
        mapping_present: true,
        dynamic_width: true,
        dynamic_precision: true,
        file: file("raw_format_segment/input.py"),
        span: span((2, 9), (2, 28)),
    },
    RawFormatSegmentExpectedRow {
        id: node(18),
        segment: node(20),
        segment_ordinal: number(0),
        dialect: name("percent-string"),
        outcome: name("malformed"),
        error_category: name("unsupported-format-character"),
        error_character: name("n"),
        error_index: number(2),
        positional_consumption: number(0),
        mapping_present: false,
        dynamic_width: false,
        dynamic_precision: false,
        file: file("raw_format_segment/input.py"),
        span: span((3, 1), (3, 7)),
    },
];

/// Exact expected facts for the coverage fixture; no facet exemptions are declared.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
