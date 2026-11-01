// The fixture sample intentionally contains no interpolated strings.

use ryo::languages::contract::{Expected, InterpolatedStringLiteralExpectedRow, expected_rows};

/// The expected rows for the canonical relation sample.
const ROWS: &[InterpolatedStringLiteralExpectedRow] = &[];

/// Return the sample’s typed reader expectation.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
