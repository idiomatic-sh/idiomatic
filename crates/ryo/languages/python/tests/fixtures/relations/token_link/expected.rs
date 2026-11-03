// Exact IDs/spans reuse the independently frozen syntax_tree/input.py fixture.
use ryo::languages::contract::{
    Expected, TokenLinkExpectedRow, expected_rows, file, name, node, span,
};
const ROWS: &[TokenLinkExpectedRow] = &[
    TokenLinkExpectedRow {
        source: node(2),
        target: node(5),
        link_kind: name("next-token"),
        file: file("token_link/input.py"),
        span: span((1, 1), (1, 4)),
    },
    TokenLinkExpectedRow {
        source: node(5),
        target: node(7),
        link_kind: name("next-token"),
        file: file("token_link/input.py"),
        span: span((1, 5), (1, 6)),
    },
    TokenLinkExpectedRow {
        source: node(7),
        target: node(11),
        link_kind: name("next-token"),
        file: file("token_link/input.py"),
        span: span((1, 6), (1, 7)),
    },
    TokenLinkExpectedRow {
        source: node(11),
        target: node(12),
        link_kind: name("next-token"),
        file: file("token_link/input.py"),
        span: span((1, 7), (1, 8)),
    },
    TokenLinkExpectedRow {
        source: node(12),
        target: node(13),
        link_kind: name("next-token"),
        file: file("token_link/input.py"),
        span: span((1, 8), (1, 9)),
    },
    TokenLinkExpectedRow {
        source: node(13),
        target: node(14),
        link_kind: name("next-token"),
        file: file("token_link/input.py"),
        span: span((1, 9), (1, 10)),
    },
    TokenLinkExpectedRow {
        source: node(14),
        target: node(15),
        link_kind: name("next-token"),
        file: file("token_link/input.py"),
        span: span((1, 10), (2, 1)),
    },
    TokenLinkExpectedRow {
        source: node(15),
        target: node(17),
        link_kind: name("next-token"),
        file: file("token_link/input.py"),
        span: span((2, 1), (2, 5)),
    },
    TokenLinkExpectedRow {
        source: node(17),
        target: node(21),
        link_kind: name("next-token"),
        file: file("token_link/input.py"),
        span: span((2, 5), (2, 11)),
    },
    TokenLinkExpectedRow {
        source: node(21),
        target: node(23),
        link_kind: name("next-token"),
        file: file("token_link/input.py"),
        span: span((2, 12), (2, 13)),
    },
    TokenLinkExpectedRow {
        source: node(23),
        target: node(26),
        link_kind: name("next-token"),
        file: file("token_link/input.py"),
        span: span((2, 14), (2, 15)),
    },
    TokenLinkExpectedRow {
        source: node(26),
        target: node(28),
        link_kind: name("next-token"),
        file: file("token_link/input.py"),
        span: span((2, 16), (2, 17)),
    },
    TokenLinkExpectedRow {
        source: node(28),
        target: node(29),
        link_kind: name("next-token"),
        file: file("token_link/input.py"),
        span: span((2, 19), (2, 29)),
    },
    TokenLinkExpectedRow {
        source: node(29),
        target: node(30),
        link_kind: name("next-token"),
        file: file("token_link/input.py"),
        span: span((2, 29), (3, 1)),
    },
    TokenLinkExpectedRow {
        source: node(7),
        target: node(12),
        link_kind: name("matching-delimiter"),
        file: file("token_link/input.py"),
        span: span((1, 6), (1, 7)),
    },
    TokenLinkExpectedRow {
        source: node(11),
        target: node(7),
        link_kind: name("enclosing-delimiter"),
        file: file("token_link/input.py"),
        span: span((1, 7), (1, 8)),
    },
];
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
