// Preserve field order and mapping keys; malformed and escaped-only strings contribute no fields.

use ryo::languages::contract::{
    Expected, FormatFieldExpectedRow, expected_rows, file, name, node, number, span,
};

/// Independently specified full rows, including negative/default values and real source positions.
const ROWS: &[FormatFieldExpectedRow] = &[
    FormatFieldExpectedRow {
        id: node(2),
        ordinal: number(0),
        argument_kind: name("positional"),
        mapping_key: name(""),
        conversion: name("s"),
        width_mode: name("none"),
        precision_mode: name("none"),
        file: file("format_field/input.py"),
        span: span((1, 1), (1, 5)),
    },
    FormatFieldExpectedRow {
        id: node(7),
        ordinal: number(0),
        argument_kind: name("mapping"),
        mapping_key: name("key"),
        conversion: name("s"),
        width_mode: name("none"),
        precision_mode: name("none"),
        file: file("format_field/input.py"),
        span: span((2, 1), (2, 22)),
    },
    FormatFieldExpectedRow {
        id: node(7),
        ordinal: number(1),
        argument_kind: name("positional"),
        mapping_key: name(""),
        conversion: name("r"),
        width_mode: name("star"),
        precision_mode: name("star"),
        file: file("format_field/input.py"),
        span: span((2, 1), (2, 22)),
    },
    FormatFieldExpectedRow {
        id: node(7),
        ordinal: number(2),
        argument_kind: name("positional"),
        mapping_key: name(""),
        conversion: name("s"),
        width_mode: name("none"),
        precision_mode: name("none"),
        file: file("format_field/input.py"),
        span: span((2, 1), (2, 22)),
    },
];

/// Exact expected facts for the coverage fixture; no facet exemptions are declared.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
