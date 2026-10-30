// The rows `relations/inherits/input.py` must give.

use ryo::languages::contract::{
    Expected, InheritsExpectedRow, expected_rows, file, name, node, span, unknown,
};

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(&[
        InheritsExpectedRow {
            occurrence: node(22),
            declaration: node(15),
            target: name("Base"),
            file: file("inherits/input.py"),
            span: span((5, 13), (5, 17)),
        },
        InheritsExpectedRow {
            occurrence: node(26),
            declaration: node(15),
            target: name("mixins.Logging"),
            file: file("inherits/input.py"),
            span: span((5, 19), (5, 33)),
        },
        InheritsExpectedRow {
            occurrence: node(49),
            declaration: node(42),
            target: name("Generic"),
            file: file("inherits/input.py"),
            span: span((9, 13), (9, 23)),
        },
        InheritsExpectedRow {
            occurrence: node(58),
            declaration: node(42),
            target: name("Protocol"),
            file: file("inherits/input.py"),
            span: span((9, 25), (9, 33)),
        },
        InheritsExpectedRow {
            occurrence: node(77),
            declaration: node(70),
            target: name("Base"),
            file: file("inherits/input.py"),
            span: span((13, 12), (13, 16)),
        },
        InheritsExpectedRow {
            occurrence: node(104),
            declaration: node(97),
            target: unknown("dynamic-base"),
            file: file("inherits/input.py"),
            span: span((17, 15), (17, 23)),
        },
        InheritsExpectedRow {
            occurrence: node(113),
            declaration: node(97),
            target: unknown("dynamic-base"),
            file: file("inherits/input.py"),
            span: span((17, 25), (17, 36)),
        },
        InheritsExpectedRow {
            occurrence: node(121),
            declaration: node(97),
            target: unknown("star-base"),
            file: file("inherits/input.py"),
            span: span((17, 38), (17, 44)),
        },
    ])
}
