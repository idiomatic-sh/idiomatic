//! The support of the `self_join_recogniser` cases.
use ryo_execution_domain::domain::self_join_recogniser::types::Decline;
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, Relation},
};
use ryo_values::cases::{Case, CaseName, Expect, Word};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as Source, JoinColumn, Operand, PlanComparison,
    RepeatedRows, Side, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};

/// A scenario: what a row asks of the part.
pub(crate) type Input = fn() -> Result<Value, Failure>;

/// What a scenario answers.
pub(crate) type Value = ();

/// This part has no failure of its own.
pub(crate) type Failure = ();

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

/// A second rule that observes the pair-multiplicity bag directly makes the select's publication
/// bag-sensitive, so the local shape is declined and never replaced.
pub(super) fn a_second_rules_bag_sensitive_terminal_select_declines_the_shape()
-> Result<Value, Failure> {
    let first = fixture();
    let mut second = first.clone();
    second.steps.truncate(3);
    second.emit.step = StepId(2); // Direct observation of the pair-multiplicity bag.
    let plans = [first, second];
    let request = crate::application::file_dag::service::prepare(
        &plans,
        &[vec![], vec![]],
        &[],
        std::sync::Arc::new(ryo_execution_store::outbound::derived::factory::semantics()),
    )
    .unwrap();
    assert!(
        request
            .ordered_existence
            .iter()
            .all(|slot| slot.get().is_none()),
        "no rule's ordered existence is compiled before its regions are resolved"
    );
    let graph =
        ryo_execution_domain::domain::file_program::service::compile_prepared(&plans, &request);
    assert_eq!(
        request.ordered_existence[0]
            .get()
            .map(|program| program.shapes.len()),
        Some(1),
        "local shape alone is insufficient"
    );
    use ryo_execution_domain::domain::file_program::types::ResolvedAlternative;
    assert!(graph.ordered_existence.iter().any(|alternative| matches!(
        alternative,
        Some(ResolvedAlternative::Declined(Decline::ExtraConsumer))
    )));
    assert!(
        graph
            .ordered_existence
            .iter()
            .all(|alternative| !matches!(alternative, Some(ResolvedAlternative::Eligible(_)))),
        "canonical Select also has a bag-sensitive terminal observer; publication cannot be replaced"
    );
    Ok(())
}

/// The rows of this part's cases.
pub(crate) const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("request_binding_rejects_a_second_rules_bag_sensitive_terminal_select"),
    word: Word::Succeeds,
    input: || a_second_rules_bag_sensitive_terminal_select_declines_the_shape,
    expect: Expect::Value(()),
}];

pub(crate) fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}
