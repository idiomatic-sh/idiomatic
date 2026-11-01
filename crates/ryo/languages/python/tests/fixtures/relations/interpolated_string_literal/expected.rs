/// The fixture sample intentionally contains no interpolated strings.

use ryo::languages::contract::{Expected, InterpolatedStringLiteralExpectedRow, expected_rows};

const ROWS: &[InterpolatedStringLiteralExpectedRow] = &[];

pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
