// The rows `relations/expression_shape/input.ts` must give.

use ryo::languages::contract::{
    Expected, ExpressionShapeExpectedRow, expected_rows, file, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[ExpressionShapeExpectedRow] = &[
    ExpressionShapeExpectedRow {
        id: node(1),
        shape: node(15),
        file: file("expression_shape/input.ts"),
        span: span((1, 1), (1, 8)),
    },
    ExpressionShapeExpectedRow {
        id: node(2),
        shape: node(16),
        file: file("expression_shape/input.ts"),
        span: span((1, 1), (1, 7)),
    },
    ExpressionShapeExpectedRow {
        id: node(3),
        shape: node(17),
        file: file("expression_shape/input.ts"),
        span: span((1, 1), (1, 2)),
    },
    ExpressionShapeExpectedRow {
        id: node(6),
        shape: node(21),
        file: file("expression_shape/input.ts"),
        span: span((1, 3), (1, 6)),
    },
    ExpressionShapeExpectedRow {
        id: node(7),
        shape: node(22),
        file: file("expression_shape/input.ts"),
        span: span((1, 3), (1, 4)),
    },
    ExpressionShapeExpectedRow {
        id: node(10),
        shape: node(25),
        file: file("expression_shape/input.ts"),
        span: span((1, 5), (1, 6)),
    },
    ExpressionShapeExpectedRow {
        id: node(15),
        shape: node(15),
        file: file("expression_shape/input.ts"),
        span: span((2, 1), (2, 10)),
    },
    ExpressionShapeExpectedRow {
        id: node(16),
        shape: node(16),
        file: file("expression_shape/input.ts"),
        span: span((2, 1), (2, 9)),
    },
    ExpressionShapeExpectedRow {
        id: node(17),
        shape: node(17),
        file: file("expression_shape/input.ts"),
        span: span((2, 1), (2, 2)),
    },
    ExpressionShapeExpectedRow {
        id: node(21),
        shape: node(21),
        file: file("expression_shape/input.ts"),
        span: span((2, 4), (2, 7)),
    },
    ExpressionShapeExpectedRow {
        id: node(22),
        shape: node(22),
        file: file("expression_shape/input.ts"),
        span: span((2, 4), (2, 5)),
    },
    ExpressionShapeExpectedRow {
        id: node(25),
        shape: node(25),
        file: file("expression_shape/input.ts"),
        span: span((2, 6), (2, 7)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
