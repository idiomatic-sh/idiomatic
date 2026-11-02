// One keyword field retains the selector kind and source order.
use ryo::languages::contract::{
    BraceFormatFieldExpectedRow, Expected, expected_rows, file, name, node, number, span,
};
const ROWS: &[BraceFormatFieldExpectedRow] = &[
    BraceFormatFieldExpectedRow {
        id: node(2),
        ordinal: number(0),
        context: name("literal"),
        parent_ordinal: number(-1),
        selector_kind: name("keyword"),
        selector_index: number(0),
        index_overflow: false,
        selector_name: name("x"),
        file: file("brace_format_field/input.py"),
        span: span((1, 1), (1, 6)),
    },
    BraceFormatFieldExpectedRow {
        id: node(2),
        ordinal: number(0),
        context: name("literal"),
        parent_ordinal: number(-1),
        selector_kind: name("index"),
        selector_index: number(2),
        index_overflow: false,
        selector_name: name("2"),
        file: file("brace_format_field/facets.py"),
        span: span((1, 1), (1, 43)),
    },
    BraceFormatFieldExpectedRow {
        id: node(2),
        ordinal: number(1),
        context: name("literal"),
        parent_ordinal: number(-1),
        selector_kind: name("keyword"),
        selector_index: number(0),
        index_overflow: false,
        selector_name: name("name"),
        file: file("brace_format_field/facets.py"),
        span: span((1, 1), (1, 43)),
    },
    BraceFormatFieldExpectedRow {
        id: node(2),
        ordinal: number(2),
        context: name("format-spec"),
        parent_ordinal: number(1),
        selector_kind: name("keyword"),
        selector_index: number(0),
        index_overflow: false,
        selector_name: name("width"),
        file: file("brace_format_field/facets.py"),
        span: span((1, 1), (1, 43)),
    },
    BraceFormatFieldExpectedRow {
        id: node(2),
        ordinal: number(3),
        context: name("literal"),
        parent_ordinal: number(-1),
        selector_kind: name("index"),
        selector_index: number(0),
        index_overflow: true,
        selector_name: name("9223372036854775808"),
        file: file("brace_format_field/facets.py"),
        span: span((1, 1), (1, 43)),
    },
];
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
