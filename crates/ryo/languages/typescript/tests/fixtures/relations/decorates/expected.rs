// The rows `relations/decorates/input.ts` must give.

use ryo::languages::contract::{
    DecoratesExpectedRow, Expected, expected_rows, file, name, node, number, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[DecoratesExpectedRow] = &[
    DecoratesExpectedRow {
        occurrence: node(2),
        declaration: node(1),
        ordinal: number(0),
        target: name("sealed"),
        arguments: name(""),
        file: file("decorates/input.ts"),
        span: span((1, 1), (1, 8)),
    },
    DecoratesExpectedRow {
        occurrence: node(36),
        declaration: node(35),
        ordinal: number(0),
        target: name("Body"),
        arguments: name(""),
        file: file("decorates/input.ts"),
        span: span((2, 24), (2, 31)),
    },
    DecoratesExpectedRow {
        occurrence: node(55),
        declaration: node(54),
        ordinal: number(0),
        target: name("Route"),
        arguments: name("\"/x\", { method: \"get\" }"),
        file: file("decorates/input.ts"),
        span: span((3, 1), (3, 32)),
    },
    DecoratesExpectedRow {
        occurrence: node(89),
        declaration: node(88),
        ordinal: number(0),
        target: name("first"),
        arguments: name(""),
        file: file("decorates/input.ts"),
        span: span((5, 1), (5, 7)),
    },
    DecoratesExpectedRow {
        occurrence: node(94),
        declaration: node(88),
        ordinal: number(1),
        target: name("second"),
        arguments: name(""),
        file: file("decorates/input.ts"),
        span: span((5, 8), (5, 15)),
    },
    DecoratesExpectedRow {
        occurrence: node(118),
        declaration: node(117),
        ordinal: number(0),
        target: name("Comp"),
        arguments: name("1"),
        file: file("decorates/input.ts"),
        span: span((6, 1), (6, 9)),
    },
    DecoratesExpectedRow {
        occurrence: node(132),
        declaration: node(131),
        ordinal: number(0),
        target: name("dec"),
        arguments: name(""),
        file: file("decorates/input.ts"),
        span: span((7, 1), (7, 10)),
    },
    DecoratesExpectedRow {
        occurrence: node(156),
        declaration: node(155),
        ordinal: number(0),
        target: name("a.b"),
        arguments: name(""),
        file: file("decorates/input.ts"),
        span: span((8, 1), (8, 7)),
    },
    DecoratesExpectedRow {
        occurrence: node(178),
        declaration: node(177),
        ordinal: number(0),
        target: name("dec"),
        arguments: name("\"arg\", 2"),
        file: file("decorates/input.ts"),
        span: span((9, 1), (9, 24)),
    },
    DecoratesExpectedRow {
        occurrence: node(205),
        declaration: node(204),
        ordinal: number(0),
        target: name("dec"),
        arguments: name("\"a\" /* ( */"),
        file: file("decorates/input.ts"),
        span: span((10, 1), (10, 18)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
