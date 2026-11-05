//! Pure immutable selector fixture shared by domain and application tests.
use crate::domain::stored_selection::types::Selector;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Comparison, Operand, ParameterId, Test, WholeNumber,
};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use std::sync::Arc;
/// A predicate and a duplicate output map, with only its referenced bound value.
pub(crate) fn program(value: i64) -> Arc<Selector> {
    Arc::new(Selector {
        relation: Relation::Function,
        identity: Arc::new(crate::domain::table_model::types::SelectionIdentity {
            comparisons: vec![Comparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(0)),
                right: Operand::Parameter(ParameterId(0)),
                origin: None,
            }],
            bindings: vec![ParameterBinding::Number(WholeNumber(value))],
        }),
        columns: vec![ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)],
        distinct: false,
    })
}

/// A canonical Function name selection followed by a distinct file projection.
/// This only constructs metadata; providers and execution owners are absent.
pub(crate) fn plan() -> ryo_values::domain::plans::types::Plan {
    use ryo_values::domain::{
        plan_parts::types::{Input, LiteralText, QueryLiteral, RepeatedRows, StepId},
        plans::types::{Emit, Operation, Plan, PlanStep, StepScope},
    };
    Plan {
        parameters: vec![],
        observation_inputs: vec![],
        observations: vec![],
        comparison_origins: vec![],
        frontier: vec![],
        steps: vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: Input::Relation(Relation::Function),
                    comparisons: vec![Comparison {
                        test: Test::Equal,
                        left: Operand::Column(ColumnIndex(1)),
                        right: Operand::Literal(QueryLiteral::Text(LiteralText("main".into()))),
                        origin: None,
                    }],
                    kept: vec![],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Project {
                    from: Input::Step(StepId(0)),
                    columns: vec![ColumnIndex(2)],
                    distinct: RepeatedRows::Dropped,
                },
            },
        ],
        emit: Emit {
            step: StepId(1),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
    }
}
