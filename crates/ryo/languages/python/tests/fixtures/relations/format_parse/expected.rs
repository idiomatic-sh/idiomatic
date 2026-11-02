// Mapping/star arguments, malformed Unicode text and escaped percent have distinct complete summaries.

use ryo::languages::contract::{
    Expected, FormatParseExpectedRow, expected_rows, file, name, node, number, span,
};

/// Independently specified full rows, including negative/default values and real source positions.
const ROWS: &[FormatParseExpectedRow] = &[
    FormatParseExpectedRow {
        id: node(2),
        dialect: name("percent-string"),
        representation: name("decoded-string"),
        outcome: name("parsed"),
        error_category: name("none"),
        error_character: name(""),
        error_index: number(0),
        positional_consumption: number(1),
        mapping_present: false,
        dynamic_width: false,
        dynamic_precision: false,
        file: file("format_parse/input.py"),
        span: span((1, 1), (1, 5)),
    },
    FormatParseExpectedRow {
        id: node(7),
        dialect: name("percent-string"),
        representation: name("decoded-string"),
        outcome: name("parsed"),
        error_category: name("none"),
        error_character: name(""),
        error_index: number(0),
        positional_consumption: number(3),
        mapping_present: true,
        dynamic_width: true,
        dynamic_precision: true,
        file: file("format_parse/input.py"),
        span: span((2, 1), (2, 16)),
    },
    FormatParseExpectedRow {
        id: node(12),
        dialect: name("percent-string"),
        representation: name("decoded-string"),
        outcome: name("malformed"),
        error_category: name("unsupported-format-character"),
        error_character: name("n"),
        error_index: number(2),
        positional_consumption: number(0),
        mapping_present: false,
        dynamic_width: false,
        dynamic_precision: false,
        file: file("format_parse/input.py"),
        span: span((3, 1), (3, 7)),
    },
    FormatParseExpectedRow {
        id: node(17),
        dialect: name("percent-string"),
        representation: name("decoded-string"),
        outcome: name("parsed"),
        error_category: name("none"),
        error_character: name(""),
        error_index: number(0),
        positional_consumption: number(0),
        mapping_present: false,
        dynamic_width: false,
        dynamic_precision: false,
        file: file("format_parse/input.py"),
        span: span((4, 1), (4, 8)),
    },
];

/// Exact expected facts for the coverage fixture; no facet exemptions are declared.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
