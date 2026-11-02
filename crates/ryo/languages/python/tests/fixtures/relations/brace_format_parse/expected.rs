// One brace selector is parsed and linked to its original string occurrence.
use ryo::languages::contract::{
    BraceFormatParseExpectedRow, Expected, expected_rows, file, name, node, number, span,
};
const ROWS: &[BraceFormatParseExpectedRow] = &[
    BraceFormatParseExpectedRow {
        id: node(2),
        representation: name("decoded-string"),
        outcome: name("parsed"),
        error_category: name("none"),
        automatic_count: number(0),
        indexed_count: number(0),
        keyword_count: number(1),
        nested_field_count: number(0),
        file: file("brace_format_parse/input.py"),
        span: span((1, 1), (1, 6)),
    },
    BraceFormatParseExpectedRow {
        id: node(2),
        representation: name("decoded-string"),
        outcome: name("parsed"),
        error_category: name("none"),
        automatic_count: number(1),
        indexed_count: number(0),
        keyword_count: number(0),
        nested_field_count: number(0),
        file: file("brace_format_parse/automatic.py"),
        span: span((1, 1), (1, 5)),
    },
    BraceFormatParseExpectedRow {
        id: node(2),
        representation: name("decoded-string"),
        outcome: name("parsed"),
        error_category: name("none"),
        automatic_count: number(0),
        indexed_count: number(2),
        keyword_count: number(2),
        nested_field_count: number(1),
        file: file("brace_format_parse/facets.py"),
        span: span((1, 1), (1, 43)),
    },
];
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
