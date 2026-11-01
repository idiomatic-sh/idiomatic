// One parsed conversion is reported with its original literal occurrence.

use ryo::languages::contract::{
    Expected, FormatParseExpectedRow, expected_rows, file, name, node, number, span,
};

const ROWS: &[FormatParseExpectedRow] = &[FormatParseExpectedRow {
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
}];

pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
