// The parser target is configured independently of source content.

use ryo::languages::contract::{Expected, LanguageTargetExpectedRow, expected_rows, file, number};

const ROWS: &[LanguageTargetExpectedRow] = &[LanguageTargetExpectedRow {
    file: file("language_target/input.py"),
    major: number(3),
    minor: number(15),
}];

pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
