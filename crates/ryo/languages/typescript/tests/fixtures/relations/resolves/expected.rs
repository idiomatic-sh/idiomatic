// The rows `relations/resolves/input.ts` must give.

use ryo::languages::contract::{
    Expected, ResolvesExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[ResolvesExpectedRow] = &[
    ResolvesExpectedRow {
        occurrence: node(60),
        owner: node(39),
        name: name("Record"),
        definition: node(6),
        file: file("resolves/input.ts"),
        span: span((4, 11), (4, 17)),
    },
    ResolvesExpectedRow {
        occurrence: node(76),
        owner: node(39),
        name: name("Record"),
        definition: node(6),
        file: file("resolves/input.ts"),
        span: span((5, 16), (5, 22)),
    },
    ResolvesExpectedRow {
        occurrence: node(88),
        owner: node(39),
        name: name("T"),
        definition: node(46),
        file: file("resolves/input.ts"),
        span: span((6, 10), (6, 11)),
    },
    ResolvesExpectedRow {
        occurrence: node(99),
        owner: node(39),
        name: name("Local"),
        definition: node(19),
        file: file("resolves/input.ts"),
        span: span((7, 10), (7, 15)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
