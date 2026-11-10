// Falling through pass has an implicit terminal result, without an explicit return.

use ryo::languages::contract::{
    Expected, FunctionTerminalExpectedRow, expected_rows, file, name, node, span,
};

/// Independently specified complete rows for this input.
pub(super) fn expected() -> Expected {
    expected_rows(&[FunctionTerminalExpectedRow {
        id: node(1),
        outcome: name("implicit"),
        file: file("function_terminal/input.py"),
        span: span((1, 1), (1, 14)),
    }])
}
