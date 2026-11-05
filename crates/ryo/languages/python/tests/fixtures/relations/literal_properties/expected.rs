// Unicode string plus both boolean values distinguish the primitive kind from its truth flag.

use ryo::languages::contract::{
    Expected, LiteralPropertiesExpectedRow, expected_rows, file, name, node, span,
};

/// Independently specified full rows, including negative/default values and real source positions.
const ROWS: &[LiteralPropertiesExpectedRow] = &[
    LiteralPropertiesExpectedRow {
        id: node(2),
        kind: name("string"),
        zero: false,
        empty: false,
        boolean: false,
        file: file("literal_properties/input.py"),
        span: span((1, 1), (1, 5)),
    },
    LiteralPropertiesExpectedRow {
        id: node(7),
        kind: name("boolean"),
        zero: false,
        empty: false,
        boolean: true,
        file: file("literal_properties/input.py"),
        span: span((2, 1), (2, 5)),
    },
    LiteralPropertiesExpectedRow {
        id: node(11),
        kind: name("boolean"),
        zero: false,
        empty: false,
        boolean: false,
        file: file("literal_properties/input.py"),
        span: span((3, 1), (3, 6)),
    },
    LiteralPropertiesExpectedRow {
        id: node(15),
        kind: name("integer"),
        zero: true,
        empty: false,
        boolean: false,
        file: file("literal_properties/input.py"),
        span: span((4, 1), (4, 2)),
    },
    LiteralPropertiesExpectedRow {
        id: node(19),
        kind: name("string"),
        zero: false,
        empty: true,
        boolean: false,
        file: file("literal_properties/input.py"),
        span: span((5, 1), (5, 3)),
    },
];

/// Exact expected facts for the coverage fixture; no facet exemptions are declared.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
