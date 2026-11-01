use ryo::languages::contract::{Expected, ExpressionTypeExpectedRow, expected_rows};
const ROWS: &[ExpressionTypeExpectedRow] = &[];
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
