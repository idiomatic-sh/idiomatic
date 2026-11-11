//! The support of the `self_join_recogniser` cases.
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, Relation},
};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as Source, JoinColumn, Operand, PlanComparison,
    RepeatedRows, Side, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};

/// The first rule: a self join of source locations by node, kept where the left number is less
/// than the right, projected and subtracted from the source.
pub(super) fn fixture() -> Plan {
    let schema = kinds(Relation::SourceLocation).to_vec();
    let number = ColumnIndex(
        schema
            .iter()
            .position(|kind| *kind == CellKind::Number)
            .map(|at| u32::try_from(at).expect("a relation's column number fits in u32"))
            .expect("typed numeric source"),
    );
    let source = Operation::Select {
        from: Source::Relation(Relation::SourceLocation),
        comparisons: vec![],
        kept: vec![],
    };
    let join = Operation::Join {
        left: Source::Step(StepId(0)),
        right: Source::Step(StepId(0)),
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
        output: vec![
            JoinColumn {
                side: Side::Left,
                column: number,
            },
            JoinColumn {
                side: Side::Right,
                column: number,
            },
        ],
        filter: vec![],
        view: vec![],
        reach: None,
    };
    let select = Operation::Select {
        from: Source::Step(StepId(1)),
        comparisons: vec![PlanComparison {
            test: Test::LessThan,
            left: Operand::Column(ColumnIndex(0)),
            right: Operand::Column(ColumnIndex(1)),
            origin: ComparisonOrigin::Unrecorded,
        }],
        kept: vec![],
    };
    let project = Operation::Project {
        from: Source::Step(StepId(2)),
        columns: vec![ColumnIndex(0)],
        distinct: RepeatedRows::Dropped,
    };
    let anti = Operation::Antijoin {
        left: Source::Step(StepId(0)),
        right: Source::Step(StepId(3)),
        left_on: vec![number],
        right_on: vec![ColumnIndex(0)],
    };
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: vec![],
        steps: [source, join, select, project, anti]
            .into_iter()
            .map(|operation| PlanStep {
                operation,
                scope: StepScope::PerFile,
            })
            .collect(),
        comparison_origins: vec![],
        emit: Emit {
            step: StepId(4),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![],
    }
}
