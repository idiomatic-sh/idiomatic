use ryo::languages::contract::{Expected, FunctionTerminalExpectedRow, expected_rows};
const ROWS: &[FunctionTerminalExpectedRow] = &[];
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
