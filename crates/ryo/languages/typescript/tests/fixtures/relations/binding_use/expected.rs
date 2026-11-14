// The rows `relations/binding_use/input.ts` must give.

use ryo::languages::contract::{
    BindingUseExpectedRow, Expected, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[BindingUseExpectedRow] = &[
    BindingUseExpectedRow {
        reference: node(16),
        binding: node(4),
        scope: node(0),
        phase: node(0),
        context: name("write"),
        certainty: name("unique"),
        file: file("binding_use/input.ts"),
        span: span((2, 1), (2, 6)),
    },
    BindingUseExpectedRow {
        reference: node(40),
        binding: node(31),
        scope: node(35),
        phase: node(25),
        context: name("value"),
        certainty: name("unique"),
        file: file("binding_use/input.ts"),
        span: span((3, 25), (3, 29)),
    },
    BindingUseExpectedRow {
        reference: node(63),
        binding: node(51),
        scope: node(0),
        phase: node(0),
        context: name("read-write"),
        certainty: name("unique"),
        file: file("binding_use/input.ts"),
        span: span((5, 1), (5, 6)),
    },
    BindingUseExpectedRow {
        reference: node(85),
        binding: node(71),
        scope: node(0),
        phase: node(0),
        context: name("value"),
        certainty: name("alternative"),
        file: file("binding_use/input.ts"),
        span: span((8, 1), (8, 6)),
    },
    BindingUseExpectedRow {
        reference: node(85),
        binding: node(79),
        scope: node(0),
        phase: node(0),
        context: name("value"),
        certainty: name("alternative"),
        file: file("binding_use/input.ts"),
        span: span((8, 1), (8, 6)),
    },
    BindingUseExpectedRow {
        reference: node(121),
        binding: node(102),
        scope: node(0),
        phase: node(0),
        context: name("type"),
        certainty: name("unique"),
        file: file("binding_use/input.ts"),
        span: span((11, 10), (11, 11)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
