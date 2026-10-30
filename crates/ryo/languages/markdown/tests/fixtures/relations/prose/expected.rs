// The rows `relations/prose/input.md` must give.

use ryo::languages::contract::{Expected, ProseExpectedRow, expected_rows, file, name, node, span};

/// The rows, in the order the language gives them.
const ROWS: &[ProseExpectedRow] = &[
    ProseExpectedRow {
        id: node(2),
        owner: node(0),
        text: name("A title with"),
        file: file("prose/input.md"),
        span: span((1, 1), (2, 1)),
    },
    ProseExpectedRow {
        id: node(13),
        owner: node(2),
        text: name("A paragraph with strong words, broken over lines, a link and raw html."),
        file: file("prose/input.md"),
        span: span((3, 1), (5, 1)),
    },
    ProseExpectedRow {
        id: node(34),
        owner: node(2),
        text: name("a tight item"),
        file: file("prose/input.md"),
        span: span((6, 1), (7, 1)),
    },
    ProseExpectedRow {
        id: node(38),
        owner: node(2),
        text: name("another with"),
        file: file("prose/input.md"),
        span: span((7, 1), (9, 1)),
    },
    ProseExpectedRow {
        id: node(46),
        owner: node(2),
        text: name("first"),
        file: file("prose/input.md"),
        span: span((9, 4), (10, 1)),
    },
    ProseExpectedRow {
        id: node(50),
        owner: node(2),
        text: name("second paragraph of the item"),
        file: file("prose/input.md"),
        span: span((11, 4), (12, 1)),
    },
    ProseExpectedRow {
        id: node(57),
        owner: node(2),
        text: name("Name"),
        file: file("prose/input.md"),
        span: span((13, 2), (13, 8)),
    },
    ProseExpectedRow {
        id: node(62),
        owner: node(2),
        text: name("Value"),
        file: file("prose/input.md"),
        span: span((13, 9), (13, 16)),
    },
    ProseExpectedRow {
        id: node(70),
        owner: node(2),
        text: name("a"),
        file: file("prose/input.md"),
        span: span((15, 2), (15, 8)),
    },
    ProseExpectedRow {
        id: node(83),
        owner: node(2),
        text: name("quoted words"),
        file: file("prose/input.md"),
        span: span((17, 3), (18, 1)),
    },
    ProseExpectedRow {
        id: node(87),
        owner: node(2),
        text: name("A reference link."),
        file: file("prose/input.md"),
        span: span((19, 1), (20, 1)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
