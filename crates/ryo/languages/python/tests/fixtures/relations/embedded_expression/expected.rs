// Native preorder identities: module0, assignment1, target2/name3, colon4,
// whitespace5, constant6/string-part7/token8, newline9; appended name10/token11.
use ryo::languages::contract::{
    EmbeddedExpressionExpectedRow, Expected, expected_rows, file, name, node, span,
};
/// The parsed name retains the quoted carrier and precise inner source span.
const ROWS: &[EmbeddedExpressionExpectedRow] = &[EmbeddedExpressionExpectedRow {
    root: node(10),
    host: node(6),
    context: name("deferred-type-expression"),
    mapping: name("exact-content"),
    file: file("embedded_expression/input.py"),
    span: span((1, 5), (1, 6)),
}];
/// Independently specified projection provenance.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
