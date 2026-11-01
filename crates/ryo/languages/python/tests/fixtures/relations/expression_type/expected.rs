use ryo::languages::contract::{Expected, ExpressionTypeExpectedRow, expected_rows};
/// The expected rows for the canonical relation sample.
const ROWS: &[ExpressionTypeExpectedRow] = &[];
/// Return the sample’s typed reader expectation.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
