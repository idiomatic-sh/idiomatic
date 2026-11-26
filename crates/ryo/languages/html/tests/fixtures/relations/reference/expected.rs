// The rows `relations/reference/input.html` must give: each target resolved to a path of the run as the language contract resolves a
// link (`../up.html` from `reference/` is `up.html`, a fragment alone is the file's own path with it, a path from the root is the
// run's own, and a target with a scheme or a host stays as written).

use ryo::languages::contract::{
    Expected, ReferenceExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[ReferenceExpectedRow] = &[
    ReferenceExpectedRow {
        occurrence: node(38),
        owner: node(21),
        target: name("reference/guide.html#start"),
        file: file("reference/input.html"),
        span: span((1, 98), (1, 121)),
    },
    ReferenceExpectedRow {
        occurrence: node(48),
        owner: node(21),
        target: name("reference/a.png"),
        file: file("reference/input.html"),
        span: span((1, 141), (1, 152)),
    },
    ReferenceExpectedRow {
        occurrence: node(54),
        owner: node(21),
        target: name("reference/input.html#top"),
        file: file("reference/input.html"),
        span: span((1, 156), (1, 167)),
    },
    ReferenceExpectedRow {
        occurrence: node(62),
        owner: node(21),
        target: name("up.html"),
        file: file("reference/input.html"),
        span: span((1, 176), (1, 193)),
    },
    ReferenceExpectedRow {
        occurrence: node(70),
        owner: node(21),
        target: name("docs/root.html?raw=1"),
        file: file("reference/input.html"),
        span: span((1, 202), (1, 230)),
    },
    ReferenceExpectedRow {
        occurrence: node(78),
        owner: node(21),
        target: name("https://example.org/x"),
        file: file("reference/input.html"),
        span: span((1, 239), (1, 267)),
    },
    ReferenceExpectedRow {
        occurrence: node(86),
        owner: node(21),
        target: name("//cdn.example.org/l.js"),
        file: file("reference/input.html"),
        span: span((1, 276), (1, 305)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
