// The rows `relations/expression_shape/input.rs` must give.

use ryo::languages::contract::{
    Expected, ExpressionShapeExpectedRow, expected_rows, file, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[ExpressionShapeExpectedRow] = &[
    ExpressionShapeExpectedRow {
        id: node(1),
        shape: node(1),
        file: file("expression_shape/input.rs"),
        span: span((1, 1), (4, 2)),
    },
    ExpressionShapeExpectedRow {
        id: node(4),
        shape: node(4),
        file: file("expression_shape/input.rs"),
        span: span((1, 4), (1, 8)),
    },
    ExpressionShapeExpectedRow {
        id: node(6),
        shape: node(6),
        file: file("expression_shape/input.rs"),
        span: span((1, 8), (1, 15)),
    },
    ExpressionShapeExpectedRow {
        id: node(8),
        shape: node(8),
        file: file("expression_shape/input.rs"),
        span: span((1, 9), (1, 14)),
    },
    ExpressionShapeExpectedRow {
        id: node(9),
        shape: node(9),
        file: file("expression_shape/input.rs"),
        span: span((1, 9), (1, 10)),
    },
    ExpressionShapeExpectedRow {
        id: node(10),
        shape: node(10),
        file: file("expression_shape/input.rs"),
        span: span((1, 9), (1, 10)),
    },
    ExpressionShapeExpectedRow {
        id: node(14),
        shape: node(14),
        file: file("expression_shape/input.rs"),
        span: span((1, 12), (1, 14)),
    },
    ExpressionShapeExpectedRow {
        id: node(15),
        shape: node(15),
        file: file("expression_shape/input.rs"),
        span: span((1, 12), (1, 14)),
    },
    ExpressionShapeExpectedRow {
        id: node(16),
        shape: node(16),
        file: file("expression_shape/input.rs"),
        span: span((1, 12), (1, 14)),
    },
    ExpressionShapeExpectedRow {
        id: node(17),
        shape: node(17),
        file: file("expression_shape/input.rs"),
        span: span((1, 12), (1, 14)),
    },
    ExpressionShapeExpectedRow {
        id: node(21),
        shape: node(21),
        file: file("expression_shape/input.rs"),
        span: span((1, 16), (1, 23)),
    },
    ExpressionShapeExpectedRow {
        id: node(24),
        shape: node(24),
        file: file("expression_shape/input.rs"),
        span: span((1, 19), (1, 23)),
    },
    ExpressionShapeExpectedRow {
        id: node(25),
        shape: node(25),
        file: file("expression_shape/input.rs"),
        span: span((1, 19), (1, 23)),
    },
    ExpressionShapeExpectedRow {
        id: node(26),
        shape: node(26),
        file: file("expression_shape/input.rs"),
        span: span((1, 19), (1, 23)),
    },
    ExpressionShapeExpectedRow {
        id: node(27),
        shape: node(27),
        file: file("expression_shape/input.rs"),
        span: span((1, 19), (1, 23)),
    },
    ExpressionShapeExpectedRow {
        id: node(30),
        shape: node(30),
        file: file("expression_shape/input.rs"),
        span: span((1, 24), (4, 2)),
    },
    ExpressionShapeExpectedRow {
        id: node(31),
        shape: node(31),
        file: file("expression_shape/input.rs"),
        span: span((1, 24), (4, 2)),
    },
    ExpressionShapeExpectedRow {
        id: node(34),
        shape: node(34),
        file: file("expression_shape/input.rs"),
        span: span((2, 5), (2, 21)),
    },
    ExpressionShapeExpectedRow {
        id: node(35),
        shape: node(35),
        file: file("expression_shape/input.rs"),
        span: span((2, 5), (2, 20)),
    },
    ExpressionShapeExpectedRow {
        id: node(36),
        shape: node(36),
        file: file("expression_shape/input.rs"),
        span: span((2, 5), (2, 20)),
    },
    ExpressionShapeExpectedRow {
        id: node(37),
        shape: node(37),
        file: file("expression_shape/input.rs"),
        span: span((2, 5), (2, 11)),
    },
    ExpressionShapeExpectedRow {
        id: node(38),
        shape: node(38),
        file: file("expression_shape/input.rs"),
        span: span((2, 5), (2, 11)),
    },
    ExpressionShapeExpectedRow {
        id: node(39),
        shape: node(39),
        file: file("expression_shape/input.rs"),
        span: span((2, 5), (2, 11)),
    },
    ExpressionShapeExpectedRow {
        id: node(42),
        shape: node(42),
        file: file("expression_shape/input.rs"),
        span: span((2, 12), (2, 20)),
    },
    ExpressionShapeExpectedRow {
        id: node(53),
        shape: node(53),
        file: file("expression_shape/input.rs"),
        span: span((3, 5), (3, 13)),
    },
    ExpressionShapeExpectedRow {
        id: node(54),
        shape: node(65),
        file: file("expression_shape/input.rs"),
        span: span((3, 5), (3, 8)),
    },
    ExpressionShapeExpectedRow {
        id: node(56),
        shape: node(65),
        file: file("expression_shape/input.rs"),
        span: span((3, 6), (3, 7)),
    },
    ExpressionShapeExpectedRow {
        id: node(57),
        shape: node(66),
        file: file("expression_shape/input.rs"),
        span: span((3, 6), (3, 7)),
    },
    ExpressionShapeExpectedRow {
        id: node(58),
        shape: node(67),
        file: file("expression_shape/input.rs"),
        span: span((3, 6), (3, 7)),
    },
    ExpressionShapeExpectedRow {
        id: node(59),
        shape: node(68),
        file: file("expression_shape/input.rs"),
        span: span((3, 6), (3, 7)),
    },
    ExpressionShapeExpectedRow {
        id: node(65),
        shape: node(65),
        file: file("expression_shape/input.rs"),
        span: span((3, 12), (3, 13)),
    },
    ExpressionShapeExpectedRow {
        id: node(66),
        shape: node(66),
        file: file("expression_shape/input.rs"),
        span: span((3, 12), (3, 13)),
    },
    ExpressionShapeExpectedRow {
        id: node(67),
        shape: node(67),
        file: file("expression_shape/input.rs"),
        span: span((3, 12), (3, 13)),
    },
    ExpressionShapeExpectedRow {
        id: node(68),
        shape: node(68),
        file: file("expression_shape/input.rs"),
        span: span((3, 12), (3, 13)),
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
