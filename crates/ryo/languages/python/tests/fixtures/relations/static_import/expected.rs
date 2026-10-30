// The rows `relations/static_import/input.py` must give.

use ryo::languages::contract::{
    Expected, StaticImportExpectedRow, expected_rows, file, name, node, span, unknown,
};

/// The rows, in the order the language gives them.
const ROWS: &[StaticImportExpectedRow] = &[
    StaticImportExpectedRow {
        occurrence: node(4),
        owner: node(0),
        binding: name("os"),
        target: name("os"),
        file: file("static_import/input.py"),
        span: span((1, 8), (1, 10)),
    },
    StaticImportExpectedRow {
        occurrence: node(11),
        owner: node(0),
        binding: name("os"),
        target: name("os.path"),
        file: file("static_import/input.py"),
        span: span((2, 8), (2, 15)),
    },
    StaticImportExpectedRow {
        occurrence: node(20),
        owner: node(0),
        binding: name("ET"),
        target: name("xml.etree.ElementTree"),
        file: file("static_import/input.py"),
        span: span((3, 8), (3, 35)),
    },
    StaticImportExpectedRow {
        occurrence: node(41),
        owner: node(0),
        binding: name("OrderedDict"),
        target: name("collections.OrderedDict"),
        file: file("static_import/input.py"),
        span: span((4, 25), (4, 36)),
    },
    StaticImportExpectedRow {
        occurrence: node(46),
        owner: node(0),
        binding: name("dd"),
        target: name("collections.defaultdict"),
        file: file("static_import/input.py"),
        span: span((4, 38), (4, 55)),
    },
    StaticImportExpectedRow {
        occurrence: node(62),
        owner: node(0),
        binding: name("sibling"),
        target: name(".sibling"),
        file: file("static_import/input.py"),
        span: span((5, 15), (5, 22)),
    },
    StaticImportExpectedRow {
        occurrence: node(75),
        owner: node(0),
        binding: name("other"),
        target: name(".pkg.thing"),
        file: file("static_import/input.py"),
        span: span((6, 18), (6, 32)),
    },
    StaticImportExpectedRow {
        occurrence: node(96),
        owner: node(0),
        binding: name("name"),
        target: name("..parent.mod.name"),
        file: file("static_import/input.py"),
        span: span((7, 26), (7, 30)),
    },
    StaticImportExpectedRow {
        occurrence: node(108),
        owner: node(0),
        binding: unknown("wildcard-import"),
        target: name("m"),
        file: file("static_import/input.py"),
        span: span((8, 15), (8, 16)),
    },
    StaticImportExpectedRow {
        occurrence: node(121),
        owner: node(0),
        binding: name("json"),
        target: name("ujson"),
        file: file("static_import/input.py"),
        span: span((11, 12), (11, 25)),
    },
    StaticImportExpectedRow {
        occurrence: node(142),
        owner: node(0),
        binding: name("json"),
        target: name("json"),
        file: file("static_import/input.py"),
        span: span((13, 12), (13, 16)),
    },
    StaticImportExpectedRow {
        occurrence: node(164),
        owner: node(0),
        binding: name("Protocol"),
        target: name("typing.Protocol"),
        file: file("static_import/input.py"),
        span: span((16, 24), (16, 32)),
    },
    StaticImportExpectedRow {
        occurrence: node(185),
        owner: node(171),
        binding: name("importlib"),
        target: name("importlib"),
        file: file("static_import/input.py"),
        span: span((20, 12), (20, 21)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
