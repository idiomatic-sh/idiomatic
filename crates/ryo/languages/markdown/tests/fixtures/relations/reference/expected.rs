// The rows `relations/reference/input.md` must give.

use ryo::languages::contract::{
    Expected, ReferenceExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[ReferenceExpectedRow] = &[
    ReferenceExpectedRow {
        occurrence: node(9),
        owner: node(2),
        target: name("guide.md#setup"),
        file: file("reference/input.md"),
        span: span((3, 5), (3, 35)),
    },
    ReferenceExpectedRow {
        occurrence: node(14),
        owner: node(2),
        target: name("reference/input.md#links"),
        file: file("reference/input.md"),
        span: span((3, 37), (3, 56)),
    },
    ReferenceExpectedRow {
        occurrence: node(19),
        owner: node(2),
        target: name("https://example.org/x"),
        file: file("reference/input.md"),
        span: span((3, 58), (3, 91)),
    },
    ReferenceExpectedRow {
        occurrence: node(24),
        owner: node(2),
        target: name("reference/img/fig.png"),
        file: file("reference/input.md"),
        span: span((3, 96), (3, 120)),
    },
    ReferenceExpectedRow {
        occurrence: node(33),
        owner: node(2),
        target: name("https://example.org/auto"),
        file: file("reference/input.md"),
        span: span((5, 10), (5, 36)),
    },
    ReferenceExpectedRow {
        occurrence: node(38),
        owner: node(2),
        target: name("reference/other.md"),
        file: file("reference/input.md"),
        span: span((5, 41), (5, 53)),
    },
    ReferenceExpectedRow {
        occurrence: node(43),
        owner: node(2),
        target: name("docs/root.md?raw=1"),
        file: file("reference/input.md"),
        span: span((5, 58), (5, 71)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
