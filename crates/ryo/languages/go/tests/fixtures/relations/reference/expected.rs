// The rows `relations/reference/input.go` must give.

use ryo::languages::contract::{
    Expected, ReferenceExpectedRow, expected_rows, file, name, node, span, unknown,
};

/// The rows, in the order the language gives them.
const ROWS: &[ReferenceExpectedRow] = &[
    ReferenceExpectedRow {
        occurrence: node(30),
        owner: node(0),
        target: name("string"),
        file: file("reference/input.go"),
        span: span((6, 7), (6, 13)),
    },
    ReferenceExpectedRow {
        occurrence: node(47),
        owner: node(35),
        target: name("int"),
        file: file("reference/input.go"),
        span: span((9, 16), (9, 19)),
    },
    ReferenceExpectedRow {
        occurrence: node(61),
        owner: node(35),
        target: name("count"),
        file: file("reference/input.go"),
        span: span((10, 11), (10, 16)),
    },
    ReferenceExpectedRow {
        occurrence: node(66),
        owner: node(35),
        target: name("limit"),
        file: file("reference/input.go"),
        span: span((10, 19), (10, 24)),
    },
    ReferenceExpectedRow {
        occurrence: node(71),
        owner: node(35),
        target: name("fmt.Println"),
        file: file("reference/input.go"),
        span: span((11, 2), (11, 13)),
    },
    ReferenceExpectedRow {
        occurrence: node(78),
        owner: node(35),
        target: name("total"),
        file: file("reference/input.go"),
        span: span((11, 14), (11, 19)),
    },
    ReferenceExpectedRow {
        occurrence: node(82),
        owner: node(35),
        target: name("config.Name.Value"),
        file: file("reference/input.go"),
        span: span((11, 21), (11, 38)),
    },
    ReferenceExpectedRow {
        occurrence: node(101),
        owner: node(35),
        target: name("Config"),
        file: file("reference/input.go"),
        span: span((12, 9), (12, 15)),
    },
    ReferenceExpectedRow {
        occurrence: node(104),
        owner: node(35),
        target: name("label"),
        file: file("reference/input.go"),
        span: span((12, 16), (12, 21)),
    },
    ReferenceExpectedRow {
        occurrence: node(108),
        owner: node(35),
        target: name("size"),
        file: file("reference/input.go"),
        span: span((12, 23), (12, 27)),
    },
    ReferenceExpectedRow {
        occurrence: node(122),
        owner: node(35),
        target: name("string"),
        file: file("reference/input.go"),
        span: span((13, 15), (13, 21)),
    },
    ReferenceExpectedRow {
        occurrence: node(125),
        owner: node(35),
        target: name("int"),
        file: file("reference/input.go"),
        span: span((13, 22), (13, 25)),
    },
    ReferenceExpectedRow {
        occurrence: node(129),
        owner: node(35),
        target: unknown("ambiguous-key"),
        file: file("reference/input.go"),
        span: span((13, 26), (13, 29)),
    },
    ReferenceExpectedRow {
        occurrence: node(138),
        owner: node(35),
        target: name("cfg.Name"),
        file: file("reference/input.go"),
        span: span((14, 2), (14, 10)),
    },
    ReferenceExpectedRow {
        occurrence: node(153),
        owner: node(35),
        target: name("obj.items"),
        file: file("reference/input.go"),
        span: span((15, 2), (15, 11)),
    },
    ReferenceExpectedRow {
        occurrence: node(169),
        owner: node(35),
        target: name("other"),
        file: file("reference/input.go"),
        span: span((15, 22), (15, 27)),
    },
    ReferenceExpectedRow {
        occurrence: node(190),
        owner: node(35),
        target: name("x"),
        file: file("reference/input.go"),
        span: span((17, 2), (17, 3)),
    },
    ReferenceExpectedRow {
        occurrence: node(195),
        owner: node(35),
        target: name("y"),
        file: file("reference/input.go"),
        span: span((17, 6), (17, 7)),
    },
    ReferenceExpectedRow {
        occurrence: node(202),
        owner: node(35),
        target: name("other"),
        file: file("reference/input.go"),
        span: span((18, 2), (18, 7)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
