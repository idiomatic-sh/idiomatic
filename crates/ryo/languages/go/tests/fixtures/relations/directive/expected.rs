// The rows `relations/directive/input.go` must give.

use ryo::languages::contract::{DirectiveExpectedRow, Expected, expected_rows, file, name, span};

/// The rows, in the order the language gives them.
const ROWS: &[DirectiveExpectedRow] = &[
    DirectiveExpectedRow {
        file: file("directive/input.go"),
        span: span((3, 1), (3, 25)),
        kind: name("go:build"),
        text: name("linux && !cgo"),
    },
    DirectiveExpectedRow {
        file: file("directive/input.go"),
        span: span((4, 1), (4, 21)),
        kind: name("+build"),
        text: name("linux,!cgo"),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
