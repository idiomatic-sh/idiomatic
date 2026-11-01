// The original carrier, including its quotes, retains its native identity.
use ryo::languages::contract::{
    Expected, LiteralExpressionParseExpectedRow, expected_rows, file, name, node, span,
};
/// One complete syntactic outcome for the original string occurrence.
const ROWS: &[LiteralExpressionParseExpectedRow] = &[LiteralExpressionParseExpectedRow {
    id: node(2),
    dialect: name("python-string-annotation"),
    outcome: name("parsed"),
    file: file("literal_expression_parse/input.py"),
    span: span((1, 1), (1, 4)),
}];
/// Return the independently specified relation fixture.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
