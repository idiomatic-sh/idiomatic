// The rows `relations/syntax_error/input.md` must give.

use ryo::languages::contract::{
    Expected, SyntaxErrorExpectedRow, expected_rows, file, name, node, span,
};

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(&[SyntaxErrorExpectedRow {
        id: node(2_147_483_648),
        message: name(
            "invalid UTF-8: a Markdown file is UTF-8 text, and the parser reads it no further",
        ),
        phase: name("parse"),
        impact: name("incomplete"),
        code: name("unspecified"),
        file: file("syntax_error/input.md"),
        span: span((3, 1), (4, 1)),
    }])
}
