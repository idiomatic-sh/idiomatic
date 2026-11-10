// The malformed signature retains a fully tokenized physical input.
// Parse invalidity is distinct from incomplete lexical coverage; neither error certifies validity.

use ryo::languages::contract::{
    Expected, SyntaxErrorExpectedRow, expected_rows, file, name, node, span,
};

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(&[
        SyntaxErrorExpectedRow {
            id: node(2_147_483_648),
            message: name("Expected a parameter or the end of the parameter list"),
            phase: name("parse"),
            impact: name("lexically-complete"),
            code: name("parser-error"),
            file: file("syntax_error/input.py"),
            span: span((5, 12), (5, 13)),
        },
        SyntaxErrorExpectedRow {
            id: node(2_147_483_649),
            message: name("Expected `)`, found newline"),
            phase: name("parse"),
            impact: name("lexically-complete"),
            code: name("parser-error"),
            file: file("syntax_error/input.py"),
            span: span((5, 13), (6, 1)),
        },
    ])
}
