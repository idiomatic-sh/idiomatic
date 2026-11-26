// Native projection reuses independently specified native syntax rows.
// The rows `relations/semantic_syntax/input.md` must give.

use ryo::languages::contract::{
    Expected, SemanticSyntaxExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[SemanticSyntaxExpectedRow] = &[
    SemanticSyntaxExpectedRow {
        id: node(0),
        kind: name("root"),
        text: name(""),
        file: file("semantic_syntax/input.md"),
        span: span((1, 1), (2, 1)),
    },
    SemanticSyntaxExpectedRow {
        id: node(1),
        kind: name("section"),
        text: name(""),
        file: file("semantic_syntax/input.md"),
        span: span((1, 1), (2, 1)),
    },
    SemanticSyntaxExpectedRow {
        id: node(2),
        kind: name("heading"),
        text: name(""),
        file: file("semantic_syntax/input.md"),
        span: span((1, 1), (2, 1)),
    },
    SemanticSyntaxExpectedRow {
        id: node(3),
        kind: name("markup"),
        text: name("# "),
        file: file("semantic_syntax/input.md"),
        span: span((1, 1), (1, 3)),
    },
    SemanticSyntaxExpectedRow {
        id: node(4),
        kind: name("emphasis"),
        text: name(""),
        file: file("semantic_syntax/input.md"),
        span: span((1, 3), (1, 6)),
    },
    SemanticSyntaxExpectedRow {
        id: node(5),
        kind: name("markup"),
        text: name("*"),
        file: file("semantic_syntax/input.md"),
        span: span((1, 3), (1, 4)),
    },
    SemanticSyntaxExpectedRow {
        id: node(6),
        kind: name("text"),
        text: name("A"),
        file: file("semantic_syntax/input.md"),
        span: span((1, 4), (1, 5)),
    },
    SemanticSyntaxExpectedRow {
        id: node(7),
        kind: name("markup"),
        text: name("*"),
        file: file("semantic_syntax/input.md"),
        span: span((1, 5), (1, 6)),
    },
    SemanticSyntaxExpectedRow {
        id: node(8),
        kind: name("text"),
        text: name(" "),
        file: file("semantic_syntax/input.md"),
        span: span((1, 6), (1, 7)),
    },
    SemanticSyntaxExpectedRow {
        id: node(9),
        kind: name("link"),
        text: name(""),
        file: file("semantic_syntax/input.md"),
        span: span((1, 7), (1, 13)),
    },
    SemanticSyntaxExpectedRow {
        id: node(10),
        kind: name("markup"),
        text: name("["),
        file: file("semantic_syntax/input.md"),
        span: span((1, 7), (1, 8)),
    },
    SemanticSyntaxExpectedRow {
        id: node(11),
        kind: name("text"),
        text: name("b"),
        file: file("semantic_syntax/input.md"),
        span: span((1, 8), (1, 9)),
    },
    SemanticSyntaxExpectedRow {
        id: node(12),
        kind: name("markup"),
        text: name("](c)"),
        file: file("semantic_syntax/input.md"),
        span: span((1, 9), (1, 13)),
    },
    SemanticSyntaxExpectedRow {
        id: node(13),
        kind: name("whitespace"),
        text: name("\n"),
        file: file("semantic_syntax/input.md"),
        span: span((1, 13), (2, 1)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
