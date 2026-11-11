//! Observer normalization preserves column mappings, bag semantics and diagnostic ownership.
use super::service::prefix;
use crate::application::rule_compilation::constants::{
    BAG_PRESERVING_PROJECTION, CALL_TEXT, COMPOSED_PREFIX, DISTINCT_IS_BARRIER, IDENTITY_SELECT,
    RELATION_ROOTED_FILTER, ZERO_WIDTH_PROJECTION,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    Applied, ColumnIndex, ComparisonOrigin, ComparisonOriginId, Function, Input as PlanInput,
    LiteralText, Operand, ParameterId, PlanComparison, QueryLiteral, RepeatedRows, StepId, Test,
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

fn observer_composes_reordered_and_repeated_projections_without_erasing_origins() {
    let first = prefix(
        &[],
        &Operation::Select {
            from: PlanInput::Relation(Relation::SemanticSyntax),
            comparisons: vec![comparison(ColumnIndex(1), ComparisonOriginId(7))],
            kept: vec![ColumnIndex(2), ColumnIndex(0), ColumnIndex(1)],
        },
    )
    .expect(RELATION_ROOTED_FILTER);
    let projected = prefix(
        &[Some(first.clone())],
        &Operation::Project {
            from: PlanInput::Step(StepId(0)),
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
            from: PlanInput::Step(StepId(1)),
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

fn observer_stops_at_distinct_and_invalid_inputs_and_distinguishes_empty_projection() {
    let select = Operation::Select {
        from: PlanInput::Relation(Relation::SemanticSyntax),
        comparisons: vec![],
        kept: vec![],
    };
    let root = prefix(&[], &select).expect(IDENTITY_SELECT);
    assert!(!root.2.is_empty());
    let empty = prefix(
        &[Some(root.clone())],
        &Operation::Project {
            from: PlanInput::Step(StepId(0)),
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
                from: PlanInput::Step(StepId(0)),
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
                from: PlanInput::Step(StepId(99)),
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
                from: PlanInput::Relation(Relation::SemanticSyntax),
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
                from: PlanInput::Relation(Relation::SemanticSyntax),
                columns: vec![ColumnIndex(99)],
                distinct: RepeatedRows::Kept,
            }
        )
        .is_none()
    );
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    ObserverComposesReorderedAndRepeatedProjectionsWithoutErasingOrigins,
    ObserverStopsAtDistinctAndInvalidInputsAndDistinguishesEmptyProjection,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "observer_composes_reordered_and_repeated_projections_without_erasing_origins",
        ),
        word: Word::Succeeds,
        input: || Scenario::ObserverComposesReorderedAndRepeatedProjectionsWithoutErasingOrigins,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "observer_stops_at_distinct_and_invalid_inputs_and_distinguishes_empty_projection",
        ),
        word: Word::Succeeds,
        input: || Scenario::ObserverStopsAtDistinctAndInvalidInputsAndDistinguishesEmptyProjection,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::ObserverComposesReorderedAndRepeatedProjectionsWithoutErasingOrigins => {
            observer_composes_reordered_and_repeated_projections_without_erasing_origins();
            Ok(())
        }
        Scenario::ObserverStopsAtDistinctAndInvalidInputsAndDistinguishesEmptyProjection => {
            observer_stops_at_distinct_and_invalid_inputs_and_distinguishes_empty_projection();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
