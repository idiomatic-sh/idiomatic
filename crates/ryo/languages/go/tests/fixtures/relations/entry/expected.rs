// The rows `relations/entry/input.go.mod` must give.

use ryo::languages::contract::{
    EntryExpectedRow, Expected, expected_rows, file, name, node, number, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[EntryExpectedRow] = &[
    EntryExpectedRow {
        decoded: name("v0.3.1"),
        key: name("example.com/a"),
        kind: name("string"),
        occurrence: node(34),
        ordinal: number(1),
        owner: node(21),
        file: file("entry/go.mod"),
        span: span((9, 2), (9, 15)),
        text: name("v0.3.1"),
        boolean: false,
        integer: number(0),
        float: name(""),
        datetime: name(""),
        radix: number(0),
    },
    EntryExpectedRow {
        decoded: name("v1.0.0"),
        key: name("example.com/one"),
        kind: name("string"),
        occurrence: node(16),
        ordinal: number(0),
        owner: node(0),
        file: file("entry/go.mod"),
        span: span((5, 9), (5, 24)),
        text: name("v1.0.0"),
        boolean: false,
        integer: number(0),
        float: name(""),
        datetime: name(""),
        radix: number(0),
    },
    EntryExpectedRow {
        decoded: name("v1.2.0"),
        key: name("example.com/b"),
        kind: name("string"),
        occurrence: node(28),
        ordinal: number(0),
        owner: node(21),
        file: file("entry/go.mod"),
        span: span((8, 2), (8, 15)),
        text: name("v1.2.0"),
        boolean: false,
        integer: number(0),
        float: name(""),
        datetime: name(""),
        radix: number(0),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
