// Native projection reuses independently specified native syntax rows.
// The rows `relations/semantic_syntax/input.css` must give.

use ryo::languages::contract::{
    Expected, SemanticSyntaxExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[SemanticSyntaxExpectedRow] = &[
    SemanticSyntaxExpectedRow {
        id: node(0),
        kind: name("stylesheet"),
        text: name(""),
        file: file("semantic_syntax/input.css"),
        span: span((1, 1), (2, 1)),
    },
    SemanticSyntaxExpectedRow {
        id: node(1),
        kind: name("qualified-rule"),
        text: name(""),
        file: file("semantic_syntax/input.css"),
        span: span((1, 1), (1, 7)),
    },
    SemanticSyntaxExpectedRow {
        id: node(2),
        kind: name("ident-token"),
        text: name("a"),
        file: file("semantic_syntax/input.css"),
        span: span((1, 1), (1, 2)),
    },
    SemanticSyntaxExpectedRow {
        id: node(3),
        kind: name("{-token"),
        text: name("{"),
        file: file("semantic_syntax/input.css"),
        span: span((1, 2), (1, 3)),
    },
    SemanticSyntaxExpectedRow {
        id: node(4),
        kind: name("declaration"),
        text: name(""),
        file: file("semantic_syntax/input.css"),
        span: span((1, 3), (1, 6)),
    },
    SemanticSyntaxExpectedRow {
        id: node(5),
        kind: name("ident-token"),
        text: name("b"),
        file: file("semantic_syntax/input.css"),
        span: span((1, 3), (1, 4)),
    },
    SemanticSyntaxExpectedRow {
        id: node(6),
        kind: name("colon-token"),
        text: name(":"),
        file: file("semantic_syntax/input.css"),
        span: span((1, 4), (1, 5)),
    },
    SemanticSyntaxExpectedRow {
        id: node(7),
        kind: name("ident-token"),
        text: name("c"),
        file: file("semantic_syntax/input.css"),
        span: span((1, 5), (1, 6)),
    },
    SemanticSyntaxExpectedRow {
        id: node(8),
        kind: name("}-token"),
        text: name("}"),
        file: file("semantic_syntax/input.css"),
        span: span((1, 6), (1, 7)),
    },
    SemanticSyntaxExpectedRow {
        id: node(9),
        kind: name("whitespace-token"),
        text: name("\n"),
        file: file("semantic_syntax/input.css"),
        span: span((1, 7), (2, 1)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
