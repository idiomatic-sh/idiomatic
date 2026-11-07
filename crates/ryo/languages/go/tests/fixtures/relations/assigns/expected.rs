// The rows `relations/assigns/input.go` must give.

use ryo::languages::contract::{AssignsExpectedRow, Expected, expected_rows, file, node, span};

/// The rows, in the order the language gives them.
const ROWS: &[AssignsExpectedRow] = &[
    AssignsExpectedRow {
        occurrence: node(9),
        binding: node(10),
        value: node(15),
        owner: node(0),
        file: file("assigns/input.go"),
        span: span((3, 14), (3, 19)),
    },
    AssignsExpectedRow {
        occurrence: node(21),
        binding: node(22),
        value: node(31),
        owner: node(0),
        file: file("assigns/input.go"),
        span: span((5, 19), (5, 22)),
    },
    AssignsExpectedRow {
        occurrence: node(54),
        binding: node(55),
        value: node(60),
        owner: node(41),
        file: file("assigns/input.go"),
        span: span((8, 14), (8, 18)),
    },
    AssignsExpectedRow {
        occurrence: node(63),
        binding: node(64),
        value: node(69),
        owner: node(41),
        file: file("assigns/input.go"),
        span: span((9, 13), (9, 18)),
    },
    AssignsExpectedRow {
        occurrence: node(90),
        binding: node(91),
        value: node(100),
        owner: node(41),
        file: file("assigns/input.go"),
        span: span((12, 17), (12, 20)),
    },
    AssignsExpectedRow {
        occurrence: node(107),
        binding: node(112),
        value: node(121),
        owner: node(41),
        file: file("assigns/input.go"),
        span: span((13, 16), (13, 21)),
    },
    AssignsExpectedRow {
        occurrence: node(130),
        binding: node(131),
        value: node(136),
        owner: node(41),
        file: file("assigns/input.go"),
        span: span((15, 9), (15, 12)),
    },
    AssignsExpectedRow {
        occurrence: node(157),
        binding: node(158),
        value: node(163),
        owner: node(147),
        file: file("assigns/input.go"),
        span: span((17, 25), (17, 29)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
