// Independently stated versioned builtin and unresolved lexical source.
use ryo::languages::contract::{
    Expected, QualifiedNameExpectedRow, expected_rows, file, name, node, number, span, unknown,
};
/// Independently specified relation rows for the fixture.
const ROWS: &[QualifiedNameExpectedRow] = &[
    QualifiedNameExpectedRow {
        id: node(2),
        path: name("builtins.ExceptionGroup"),
        provenance: name("builtin"),
        since_major: number(3),
        since_minor: number(11),
        file: file("qualified_name/input.py"),
        span: span((1, 1), (1, 15)),
    },
    QualifiedNameExpectedRow {
        id: node(6),
        path: unknown("qualified-origin"),
        provenance: name("unresolved"),
        since_major: number(0),
        since_minor: number(0),
        file: file("qualified_name/input.py"),
        span: span((2, 1), (2, 8)),
    },
];
/// A known lexical path and an explicit unknown remain distinct in shared conformance.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
