// The rows `relations/decorates/input.py` must give.

use ryo::languages::contract::{
    DecoratesExpectedRow, Expected, expected_rows, file, name, node, number, span, unknown,
};

/// The rows, in the order the language gives them.
const ROWS: &[DecoratesExpectedRow] = &[
    DecoratesExpectedRow {
        occurrence: node(2),
        declaration: node(1),
        ordinal: number(0),
        target: name("app.route"),
        arguments: name("\"/\", methods=[\"GET\"]"),
        file: file("decorates/input.py"),
        span: span((1, 1), (1, 33)),
    },
    DecoratesExpectedRow {
        occurrence: node(30),
        declaration: node(1),
        ordinal: number(1),
        target: name("login_required"),
        arguments: name(""),
        file: file("decorates/input.py"),
        span: span((2, 1), (2, 16)),
    },
    DecoratesExpectedRow {
        occurrence: node(35),
        declaration: node(1),
        ordinal: number(2),
        target: name("functools.lru_cache"),
        arguments: name("maxsize=None"),
        file: file("decorates/input.py"),
        span: span((3, 1), (3, 35)),
    },
    DecoratesExpectedRow {
        occurrence: node(71),
        declaration: node(70),
        ordinal: number(0),
        target: name("dataclass"),
        arguments: name("frozen=True"),
        file: file("decorates/input.py"),
        span: span((8, 1), (8, 24)),
    },
    DecoratesExpectedRow {
        occurrence: node(105),
        declaration: node(104),
        ordinal: number(0),
        target: unknown("dynamic-decorator"),
        arguments: name(""),
        file: file("decorates/input.py"),
        span: span((13, 1), (13, 13)),
    },
    DecoratesExpectedRow {
        occurrence: node(115),
        declaration: node(104),
        ordinal: number(1),
        target: unknown("dynamic-decorator"),
        arguments: name(""),
        file: file("decorates/input.py"),
        span: span((14, 1), (14, 13)),
    },
    DecoratesExpectedRow {
        occurrence: node(128),
        declaration: node(104),
        ordinal: number(2),
        target: unknown("dynamic-decorator"),
        arguments: name(""),
        file: file("decorates/input.py"),
        span: span((15, 1), (15, 15)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
