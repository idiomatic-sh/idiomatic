// The rows `relations/prose/input.html` must give.

use ryo::languages::contract::{Expected, ProseExpectedRow, expected_rows, file, name, node, span};

/// The rows, in the order the language gives them.
const ROWS: &[ProseExpectedRow] = &[
    ProseExpectedRow {
        id: node(11),
        owner: node(0),
        text: name("HTML"),
        file: file("prose/input.html"),
        span: span((1, 16), (1, 35)),
    },
    ProseExpectedRow {
        id: node(27),
        owner: node(21),
        text: name("Hello world."),
        file: file("prose/input.html"),
        span: span((1, 68), (1, 136)),
    },
    ProseExpectedRow {
        id: node(45),
        owner: node(21),
        text: name("Hello world"),
        file: file("prose/input.html"),
        span: span((1, 136), (1, 169)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
