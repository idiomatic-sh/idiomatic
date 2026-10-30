// The rows `relations/annotates/input.go` must give.

use ryo::languages::contract::{
    AnnotatesExpectedRow, Expected, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[AnnotatesExpectedRow] = &[
    AnnotatesExpectedRow {
        occurrence: node(23),
        owner: node(20),
        annotation: name("string"),
        file: file("annotates/input.go"),
        span: span((4, 7), (4, 13)),
    },
    AnnotatesExpectedRow {
        occurrence: node(27),
        owner: node(26),
        annotation: name("*Base"),
        file: file("annotates/input.go"),
        span: span((5, 2), (5, 7)),
    },
    AnnotatesExpectedRow {
        occurrence: node(33),
        owner: node(32),
        annotation: name("sync.Mutex"),
        file: file("annotates/input.go"),
        span: span((6, 2), (6, 12)),
    },
    AnnotatesExpectedRow {
        occurrence: node(54),
        owner: node(51),
        annotation: name("int"),
        file: file("annotates/input.go"),
        span: span((9, 16), (9, 19)),
    },
    AnnotatesExpectedRow {
        occurrence: node(62),
        owner: node(59),
        annotation: name("...string"),
        file: file("annotates/input.go"),
        span: span((9, 27), (9, 36)),
    },
    AnnotatesExpectedRow {
        occurrence: node(74),
        owner: node(71),
        annotation: name("bool"),
        file: file("annotates/input.go"),
        span: span((9, 42), (9, 46)),
    },
    AnnotatesExpectedRow {
        occurrence: node(82),
        owner: node(79),
        annotation: name("error"),
        file: file("annotates/input.go"),
        span: span((9, 52), (9, 57)),
    },
    AnnotatesExpectedRow {
        occurrence: node(97),
        owner: node(94),
        annotation: name("uint8"),
        file: file("annotates/input.go"),
        span: span((10, 12), (10, 17)),
    },
    AnnotatesExpectedRow {
        occurrence: node(117),
        owner: node(110),
        annotation: name("float64"),
        file: file("annotates/input.go"),
        span: span((11, 20), (11, 27)),
    },
    AnnotatesExpectedRow {
        occurrence: node(117),
        owner: node(114),
        annotation: name("float64"),
        file: file("annotates/input.go"),
        span: span((11, 20), (11, 27)),
    },
    AnnotatesExpectedRow {
        occurrence: node(151),
        owner: node(148),
        annotation: name("*Service"),
        file: file("annotates/input.go"),
        span: span((15, 9), (15, 17)),
    },
    AnnotatesExpectedRow {
        occurrence: node(166),
        owner: node(165),
        annotation: name("int"),
        file: file("annotates/input.go"),
        span: span((15, 28), (15, 31)),
    },
    AnnotatesExpectedRow {
        occurrence: node(171),
        owner: node(170),
        annotation: name("error"),
        file: file("annotates/input.go"),
        span: span((15, 33), (15, 38)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
