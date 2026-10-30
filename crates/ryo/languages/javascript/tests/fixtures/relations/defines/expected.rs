// The rows `relations/defines/input.js` must give.

use ryo::languages::contract::{
    DefinesExpectedRow, Expected, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[DefinesExpectedRow] = &[
    DefinesExpectedRow {
        id: node(4),
        kind: name("import"),
        refinement: name("import-default-specifier"),
        name: name("tool"),
        owner: node(0),
        file: file("defines/input.js"),
        span: span((1, 8), (1, 12)),
    },
    DefinesExpectedRow {
        id: node(10),
        kind: name("import"),
        refinement: name("import-specifier"),
        name: name("load"),
        owner: node(0),
        file: file("defines/input.js"),
        span: span((1, 23), (1, 27)),
    },
    DefinesExpectedRow {
        id: node(29),
        kind: name("import"),
        refinement: name("import-namespace-specifier"),
        name: name("path"),
        owner: node(0),
        file: file("defines/input.js"),
        span: span((2, 13), (2, 17)),
    },
    DefinesExpectedRow {
        id: node(46),
        kind: name("constant"),
        refinement: name("variable-declarator"),
        name: name("first"),
        owner: node(0),
        file: file("defines/input.js"),
        span: span((3, 8), (3, 13)),
    },
    DefinesExpectedRow {
        id: node(46),
        kind: name("constant"),
        refinement: name("variable-declarator"),
        name: name("second"),
        owner: node(0),
        file: file("defines/input.js"),
        span: span((3, 16), (3, 22)),
    },
    DefinesExpectedRow {
        id: node(74),
        kind: name("variable"),
        refinement: name("variable-declarator"),
        name: name("later"),
        owner: node(0),
        file: file("defines/input.js"),
        span: span((4, 5), (4, 10)),
    },
    DefinesExpectedRow {
        id: node(82),
        kind: name("variable"),
        refinement: name("variable-declarator"),
        name: name("old"),
        owner: node(0),
        file: file("defines/input.js"),
        span: span((5, 5), (5, 8)),
    },
    DefinesExpectedRow {
        id: node(92),
        kind: name("function"),
        refinement: name("function-declaration"),
        name: name("run"),
        owner: node(0),
        file: file("defines/input.js"),
        span: span((6, 10), (6, 13)),
    },
    DefinesExpectedRow {
        id: node(114),
        kind: name("class"),
        refinement: name("class-declaration"),
        name: name("Store"),
        owner: node(0),
        file: file("defines/input.js"),
        span: span((9, 7), (9, 12)),
    },
    DefinesExpectedRow {
        id: node(123),
        kind: name("property"),
        refinement: name("property-definition"),
        name: name("items"),
        owner: node(114),
        file: file("defines/input.js"),
        span: span((10, 3), (10, 8)),
    },
    DefinesExpectedRow {
        id: node(134),
        kind: name("property"),
        refinement: name("property-definition"),
        name: name("#secret"),
        owner: node(114),
        file: file("defines/input.js"),
        span: span((11, 3), (11, 10)),
    },
    DefinesExpectedRow {
        id: node(144),
        kind: name("method"),
        refinement: name("method-definition"),
        name: name("add"),
        owner: node(114),
        file: file("defines/input.js"),
        span: span((12, 3), (12, 6)),
    },
    DefinesExpectedRow {
        id: node(172),
        kind: name("label"),
        refinement: name("labeled-statement"),
        name: name("outer"),
        owner: node(0),
        file: file("defines/input.js"),
        span: span((15, 1), (15, 6)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
