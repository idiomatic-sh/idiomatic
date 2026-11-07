// The rows `relations/directive/input.js` must give.

use ryo::languages::contract::{DirectiveExpectedRow, Expected, expected_rows, file, name, span};

/// The rows, in the order the language gives them.
const ROWS: &[DirectiveExpectedRow] = &[
    DirectiveExpectedRow {
        file: file("directive/input.js"),
        span: span((1, 1), (1, 14)),
        kind: name("use client"),
        text: name(""),
    },
    DirectiveExpectedRow {
        file: file("directive/input.js"),
        span: span((2, 1), (2, 14)),
        kind: name("use strict"),
        text: name(""),
    },
    DirectiveExpectedRow {
        file: file("directive/input.js"),
        span: span((5, 3), (5, 16)),
        kind: name("use server"),
        text: name(""),
    },
    DirectiveExpectedRow {
        file: file("directive/input.js"),
        span: span((10, 3), (10, 16)),
        kind: name("use server"),
        text: name(""),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
