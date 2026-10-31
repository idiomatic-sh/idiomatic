// Unicode source spelling and canonical lexical identity are independently specified.
use ryo::languages::contract::{
    Expected, IdentifierExpectedRow, expected_rows, file, name, node, span,
};
/// Independently specified relation rows for the fixture.
const ROWS: &[IdentifierExpectedRow] = &[IdentifierExpectedRow {
    id: node(2),
    name: name("exec"),
    file: file("identifier/input.py"),
    span: span((1, 1), (1, 17)),
}];
/// One canonical identifier at its original Unicode source occurrence.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
