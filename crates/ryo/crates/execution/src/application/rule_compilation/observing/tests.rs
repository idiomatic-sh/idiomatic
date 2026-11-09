//! Observer normalization preserves column mappings, bag semantics and diagnostic ownership.
use super::prefix;
use crate::application::rule_compilation::constants::{
    BAG_PRESERVING_PROJECTION, CALL_TEXT, COMPOSED_PREFIX, DISTINCT_IS_BARRIER, IDENTITY_SELECT,
    RELATION_ROOTED_FILTER, ZERO_WIDTH_PROJECTION,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    Applied, ColumnIndex, ComparisonOrigin, ComparisonOriginId, Function, Input, LiteralText,
    Operand, ParameterId, PlanComparison, QueryLiteral, RepeatedRows, StepId, Test,
};
use ryo_values::domain::plans::types::Operation;

/// A typed symbol predicate with independent source-origin ownership.
fn comparison(column: ColumnIndex, origin: ComparisonOriginId) -> PlanComparison {
    PlanComparison {
        test: Test::Equal,
        left: Operand::Column(column),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(CALL_TEXT.into()))),
        origin: ComparisonOrigin::Recorded(origin),
    }
}

#[test]
fn observer_composes_reordered_and_repeated_projections_without_erasing_origins() {
    let first = prefix(
        &[],
        &Operation::Select {
            from: Input::Relation(Relation::SemanticSyntax),
            comparisons: vec![comparison(ColumnIndex(1), ComparisonOriginId(7))],
            kept: vec![ColumnIndex(2), ColumnIndex(0), ColumnIndex(1)],
        },
    )
    .expect(RELATION_ROOTED_FILTER);
    let projected = prefix(
        &[Some(first.clone())],
        &Operation::Project {
            from: Input::Step(StepId(0)),
            columns: vec![
                ColumnIndex(1),
                ColumnIndex(2),
                ColumnIndex(0),
                ColumnIndex(2),
            ],
            distinct: RepeatedRows::Kept,
        },
    )
    .expect(BAG_PRESERVING_PROJECTION);
    assert_eq!(
        projected.2,
        [
            ColumnIndex(0),
            ColumnIndex(1),
            ColumnIndex(2),
            ColumnIndex(1)
        ]
    );
    let mut tested = comparison(ColumnIndex(1), ComparisonOriginId(99));
    tested.left = Operand::Function(Applied {
        function: Function::Concat,
        arguments: vec![
            Operand::Column(ColumnIndex(3)),
            Operand::Column(ColumnIndex(1)),
        ],
    });
    tested.right = Operand::Parameter(ParameterId(0));
    let result = prefix(
        &[Some(first), Some(projected)],
        &Operation::Select {
            from: Input::Step(StepId(1)),
            comparisons: vec![tested],
            kept: vec![ColumnIndex(2), ColumnIndex(0)],
        },
    )
    .expect(COMPOSED_PREFIX);
    assert_eq!(result.0, Relation::SemanticSyntax);
    assert_eq!(result.2, [ColumnIndex(2), ColumnIndex(0)]);
    assert_eq!(
        result.1[0].origin,
        ComparisonOrigin::Recorded(ComparisonOriginId(7))
    );
    assert_eq!(
        result.1[1].origin,
        ComparisonOrigin::Recorded(ComparisonOriginId(99))
    );
    assert_eq!(result.1[1].right, Operand::Parameter(ParameterId(0)));
    assert_eq!(
        result.1[1].left,
        Operand::Function(Applied {
            function: Function::Concat,
            arguments: vec![
                Operand::Column(ColumnIndex(1)),
                Operand::Column(ColumnIndex(1))
            ],
        })
    );
}

#[test]
fn observer_stops_at_distinct_and_invalid_inputs_and_distinguishes_empty_projection() {
    let select = Operation::Select {
        from: Input::Relation(Relation::SemanticSyntax),
        comparisons: vec![],
        kept: vec![],
    };
    let root = prefix(&[], &select).expect(IDENTITY_SELECT);
    assert!(!root.2.is_empty());
    let empty = prefix(
        &[Some(root.clone())],
        &Operation::Project {
            from: Input::Step(StepId(0)),
            columns: vec![],
            distinct: RepeatedRows::Kept,
        },
    )
    .expect(ZERO_WIDTH_PROJECTION);
    assert!(empty.2.is_empty());
    assert!(
        prefix(
            &[Some(root)],
            &Operation::Project {
                from: Input::Step(StepId(0)),
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Dropped,
            }
        )
        .is_none(),
        "{DISTINCT_IS_BARRIER}"
    );
    assert!(
        prefix(
            &[],
            &Operation::Select {
                from: Input::Step(StepId(99)),
                comparisons: vec![],
                kept: vec![],
            }
        )
        .is_none()
    );
    assert!(
        prefix(
            &[],
            &Operation::Select {
                from: Input::Relation(Relation::SemanticSyntax),
                comparisons: vec![comparison(ColumnIndex(99), ComparisonOriginId(7))],
                kept: vec![],
            }
        )
        .is_none()
    );
    assert!(
        prefix(
            &[],
            &Operation::Project {
                from: Input::Relation(Relation::SemanticSyntax),
                columns: vec![ColumnIndex(99)],
                distinct: RepeatedRows::Kept,
            }
        )
        .is_none()
    );
}
