// The rows `relations/syntax_error/input.rs` must give.

use ryo::languages::contract::{
    Expected, SyntaxErrorExpectedRow, expected_rows, file, name, node, span,
};

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(&[
        SyntaxErrorExpectedRow {
            id: node(2_147_483_648),
            message: name("expected a name"),
            phase: name("parse"),
            impact: name("incomplete"),
            code: name("unspecified"),
            file: file("syntax_error/input.rs"),
            span: span((2, 1), (2, 1)),
        },
        SyntaxErrorExpectedRow {
            id: node(2_147_483_649),
            message: name("expected function arguments"),
            phase: name("parse"),
            impact: name("incomplete"),
            code: name("unspecified"),
            file: file("syntax_error/input.rs"),
            span: span((2, 1), (2, 1)),
        },
        SyntaxErrorExpectedRow {
            id: node(2_147_483_650),
            message: name("expected a block"),
            phase: name("parse"),
            impact: name("incomplete"),
            code: name("unspecified"),
            file: file("syntax_error/input.rs"),
            span: span((2, 1), (2, 1)),
        },
    ])
}
