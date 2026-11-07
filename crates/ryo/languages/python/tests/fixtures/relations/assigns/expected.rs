// The rows `relations/assigns/input.py` must give.

use ryo::languages::contract::{AssignsExpectedRow, Expected, expected_rows, file, node, span};

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(&[
        AssignsExpectedRow {
            occurrence: node(1),
            binding: node(2),
            value: node(7),
            owner: node(0),
            file: file("assigns/input.py"),
            span: span((1, 8), (1, 17)),
        },
        AssignsExpectedRow {
            occurrence: node(11),
            binding: node(12),
            value: node(22),
            owner: node(0),
            file: file("assigns/input.py"),
            span: span((2, 18), (2, 26)),
        },
        AssignsExpectedRow {
            occurrence: node(11),
            binding: node(17),
            value: node(22),
            owner: node(0),
            file: file("assigns/input.py"),
            span: span((2, 18), (2, 26)),
        },
        AssignsExpectedRow {
            occurrence: node(26),
            binding: node(27),
            value: node(36),
            owner: node(0),
            file: file("assigns/input.py"),
            span: span((3, 14), (3, 21)),
        },
        AssignsExpectedRow {
            occurrence: node(84),
            binding: node(85),
            value: node(90),
            owner: node(0),
            file: file("assigns/input.py"),
            span: span((7, 12), (7, 21)),
        },
        AssignsExpectedRow {
            occurrence: node(126),
            binding: node(127),
            value: node(132),
            owner: node(115),
            file: file("assigns/input.py"),
            span: span((12, 12), (12, 18)),
        },
        AssignsExpectedRow {
            occurrence: node(141),
            binding: node(142),
            value: node(147),
            owner: node(115),
            file: file("assigns/input.py"),
            span: span((13, 19), (13, 27)),
        },
    ])
}
