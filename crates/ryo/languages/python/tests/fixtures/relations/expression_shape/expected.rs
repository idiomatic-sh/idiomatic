// Two source occurrences share one opaque parsed expression class.
use ryo::languages::contract::{
    Expected, ExpressionShapeExpectedRow, expected_rows, file, node, span,
};
const ROWS: &[ExpressionShapeExpectedRow] = &[
    ExpressionShapeExpectedRow {
        id: node(2),
        shape: node(6),
        file: file("expression_shape/input.py"),
        span: span((1, 1), (1, 2)),
    },
    ExpressionShapeExpectedRow {
        id: node(6),
        shape: node(6),
        file: file("expression_shape/input.py"),
        span: span((2, 1), (2, 2)),
    },
];
/// Original occurrence positions remain separate from their structural class.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
