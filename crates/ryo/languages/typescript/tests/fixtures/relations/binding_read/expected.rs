// The rows `relations/binding_read/input.ts` must give.

use ryo::languages::contract::{
    BindingReadExpectedRow, Expected, expected_rows, file, name, node, number, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[BindingReadExpectedRow] = &[
    BindingReadExpectedRow {
        reference: node(4),
        name: name("Math"),
        scope: node(0),
        phase: node(0),
        context: name("value"),
        state: name("builtin"),
        since_major: number(0),
        since_minor: number(0),
        file: file("binding_read/input.ts"),
        span: span((1, 1), (1, 5)),
    },
    BindingReadExpectedRow {
        reference: node(22),
        name: name("window"),
        scope: node(0),
        phase: node(0),
        context: name("value"),
        state: name("unbound"),
        since_major: number(0),
        since_minor: number(0),
        file: file("binding_read/input.ts"),
        span: span((2, 1), (2, 7)),
    },
    BindingReadExpectedRow {
        reference: node(60),
        name: name("outer"),
        scope: node(54),
        phase: node(46),
        context: name("value"),
        state: name("unique-declaration"),
        since_major: number(0),
        since_minor: number(0),
        file: file("binding_read/input.ts"),
        span: span((4, 26), (4, 31)),
    },
    BindingReadExpectedRow {
        reference: node(83),
        name: name("twice"),
        scope: node(0),
        phase: node(0),
        context: name("value"),
        state: name("alternative-declarations"),
        since_major: number(0),
        since_minor: number(0),
        file: file("binding_read/input.ts"),
        span: span((7, 1), (7, 6)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
