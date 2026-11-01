use ryo::languages::contract::{Expected, FunctionTerminalExpectedRow, expected_rows};
/// The expected rows for the canonical relation sample.
const ROWS: &[FunctionTerminalExpectedRow] = &[];
/// Return the sample’s typed reader expectation.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
