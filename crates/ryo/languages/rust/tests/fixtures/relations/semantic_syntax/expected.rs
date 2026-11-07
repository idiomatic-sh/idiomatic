// Native projection reuses independently specified native syntax rows.
// The rows `relations/semantic_syntax/input.rs` must give.

use ryo::languages::contract::{
    Expected, SemanticSyntaxExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[SemanticSyntaxExpectedRow] = &[
    SemanticSyntaxExpectedRow {
        id: node(0),
        kind: name("source-file"),
        text: name(""),
        file: file("semantic_syntax/input.rs"),
        span: span((1, 1), (2, 1)),
    },
    SemanticSyntaxExpectedRow {
        id: node(1),
        kind: name("fn"),
        text: name(""),
        file: file("semantic_syntax/input.rs"),
        span: span((1, 1), (1, 12)),
    },
    SemanticSyntaxExpectedRow {
        id: node(2),
        kind: name("fn-kw"),
        text: name("fn"),
        file: file("semantic_syntax/input.rs"),
        span: span((1, 1), (1, 3)),
    },
    SemanticSyntaxExpectedRow {
        id: node(3),
        kind: name("whitespace"),
        text: name(" "),
        file: file("semantic_syntax/input.rs"),
        span: span((1, 3), (1, 4)),
    },
    SemanticSyntaxExpectedRow {
        id: node(4),
        kind: name("name"),
        text: name(""),
        file: file("semantic_syntax/input.rs"),
        span: span((1, 4), (1, 7)),
    },
    SemanticSyntaxExpectedRow {
        id: node(5),
        kind: name("ident"),
        text: name("run"),
        file: file("semantic_syntax/input.rs"),
        span: span((1, 4), (1, 7)),
    },
    SemanticSyntaxExpectedRow {
        id: node(6),
        kind: name("param-list"),
        text: name(""),
        file: file("semantic_syntax/input.rs"),
        span: span((1, 7), (1, 9)),
    },
    SemanticSyntaxExpectedRow {
        id: node(7),
        kind: name("l-paren"),
        text: name("("),
        file: file("semantic_syntax/input.rs"),
        span: span((1, 7), (1, 8)),
    },
    SemanticSyntaxExpectedRow {
        id: node(8),
        kind: name("r-paren"),
        text: name(")"),
        file: file("semantic_syntax/input.rs"),
        span: span((1, 8), (1, 9)),
    },
    SemanticSyntaxExpectedRow {
        id: node(9),
        kind: name("whitespace"),
        text: name(" "),
        file: file("semantic_syntax/input.rs"),
        span: span((1, 9), (1, 10)),
    },
    SemanticSyntaxExpectedRow {
        id: node(10),
        kind: name("block-expr"),
        text: name(""),
        file: file("semantic_syntax/input.rs"),
        span: span((1, 10), (1, 12)),
    },
    SemanticSyntaxExpectedRow {
        id: node(11),
        kind: name("stmt-list"),
        text: name(""),
        file: file("semantic_syntax/input.rs"),
        span: span((1, 10), (1, 12)),
    },
    SemanticSyntaxExpectedRow {
        id: node(12),
        kind: name("l-curly"),
        text: name("{"),
        file: file("semantic_syntax/input.rs"),
        span: span((1, 10), (1, 11)),
    },
    SemanticSyntaxExpectedRow {
        id: node(13),
        kind: name("r-curly"),
        text: name("}"),
        file: file("semantic_syntax/input.rs"),
        span: span((1, 11), (1, 12)),
    },
    SemanticSyntaxExpectedRow {
        id: node(14),
        kind: name("whitespace"),
        text: name("\n"),
        file: file("semantic_syntax/input.rs"),
        span: span((1, 12), (2, 1)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
