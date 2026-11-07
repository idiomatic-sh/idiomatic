// The rows `relations/annotates/input.py` must give.

use ryo::languages::contract::{
    AnnotatesExpectedRow, Expected, expected_rows, file, name, node, span,
};

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(&[
        AnnotatesExpectedRow {
            occurrence: node(28),
            owner: node(23),
            annotation: name("str"),
            file: file("annotates/input.py"),
            span: span((4, 17), (4, 20)),
        },
        AnnotatesExpectedRow {
            occurrence: node(38),
            owner: node(33),
            annotation: name("int"),
            file: file("annotates/input.py"),
            span: span((4, 29), (4, 32)),
        },
        AnnotatesExpectedRow {
            occurrence: node(53),
            owner: node(47),
            annotation: name("\"Extra\""),
            file: file("annotates/input.py"),
            span: span((4, 45), (4, 52)),
        },
        AnnotatesExpectedRow {
            occurrence: node(64),
            owner: node(58),
            annotation: name("Optional[int]"),
            file: file("annotates/input.py"),
            span: span((4, 62), (4, 75)),
        },
        AnnotatesExpectedRow {
            occurrence: node(75),
            owner: node(15),
            annotation: name("list[str]"),
            file: file("annotates/input.py"),
            span: span((4, 80), (4, 89)),
        },
        AnnotatesExpectedRow {
            occurrence: node(90),
            owner: node(86),
            annotation: name("list[str]"),
            file: file("annotates/input.py"),
            span: span((5, 13), (5, 22)),
        },
        AnnotatesExpectedRow {
            occurrence: node(110),
            owner: node(106),
            annotation: name("int"),
            file: file("annotates/input.py"),
            span: span((6, 12), (6, 15)),
        },
        AnnotatesExpectedRow {
            occurrence: node(136),
            owner: node(132),
            annotation: name("bool"),
            file: file("annotates/input.py"),
            span: span((11, 12), (11, 16)),
        },
        AnnotatesExpectedRow {
            occurrence: node(150),
            owner: node(146),
            annotation: name("\"pathlib.Path\""),
            file: file("annotates/input.py"),
            span: span((12, 11), (12, 25)),
        },
    ])
}
