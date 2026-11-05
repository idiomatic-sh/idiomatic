use super::*;
use ryo_values::domain::plan_parts::types::{Comparison, JoinColumn};
use ryo_values::domain::plans::types::{Emit, PlanStep};

fn fixture() -> (Plan, Vec<CellKind>) {
    let schema = kinds(ryo_relations::domain::relations::types::Relation::SourceLocation).to_vec();
    let number = ColumnIndex(
        schema
            .iter()
            .position(|kind| *kind == CellKind::Number)
            .expect("typed numeric source") as u32,
    );
    let source = Operation::Select {
        from: Input::Relation(ryo_relations::domain::relations::types::Relation::SourceLocation),
        comparisons: vec![],
        kept: vec![],
    };
    let join = Operation::Join {
        left: Input::Step(StepId(0)),
        right: Input::Step(StepId(0)),
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
        from: Input::Step(StepId(1)),
        comparisons: vec![Comparison {
            test: Test::LessThan,
            left: Operand::Column(ColumnIndex(0)),
            right: Operand::Column(ColumnIndex(1)),
            origin: None,
        }],
        kept: vec![],
    };
    let project = Operation::Project {
        from: Input::Step(StepId(2)),
        columns: vec![ColumnIndex(0)],
        distinct: RepeatedRows::Dropped,
    };
    let anti = Operation::Antijoin {
        left: Input::Step(StepId(0)),
        right: Input::Step(StepId(3)),
        left_on: vec![number],
        right_on: vec![ColumnIndex(0)],
    };
    (
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
        },
        schema,
    )
}
fn prepared(plan: &Plan, schema: &[CellKind]) -> Program {
    compile(plan, &|id| (id == StepId(0)).then_some(schema))
}
#[test]
fn eligible_shape_names_original_source_replacements_and_retained_operators() {
    let (plan, schema) = fixture();
    let found = prepared(&plan, &schema);
    assert_eq!(found.shapes.len(), 1);
    let shape = &found.shapes[0];
    assert_eq!(
        (
            shape.source,
            shape.replaced_join,
            shape.replaced_select,
            shape.retained_project,
            shape.retained_antijoin
        ),
        (StepId(0), StepId(1), StepId(2), StepId(3), StepId(4))
    );
    assert_eq!(shape.extremum, Extremum::Maximum);
    assert!(!shape.number.origins.is_empty());
}
#[test]
fn rejects_extra_consumers_filters_and_non_strict_or_mapped_views() {
    let (base, schema) = fixture();
    let mut plan = base.clone();
    plan.frontier.push(Input::Step(StepId(1)));
    assert_eq!(
        prepared(&plan, &schema).declines[0].1,
        Decline::ExtraConsumer
    );
    let mut plan = base.clone();
    let comparison = match &plan.steps[2].operation {
        Operation::Select { comparisons, .. } => comparisons[0].clone(),
        _ => unreachable!(),
    };
    if let Operation::Join { filter, .. } = &mut plan.steps[1].operation {
        filter.push(comparison);
    }
    assert_eq!(
        prepared(&plan, &schema).declines[0].1,
        Decline::NotPlainSelfJoin
    );
    let mut plan = base.clone();
    if let Operation::Join { view, .. } = &mut plan.steps[1].operation {
        view.push(ColumnIndex(0));
    }
    assert_eq!(
        prepared(&plan, &schema).declines[0].1,
        Decline::NotPlainSelfJoin
    );
    let mut plan = base;
    if let Operation::Select { comparisons, .. } = &mut plan.steps[2].operation {
        comparisons[0].test = Test::NaturalLessThan;
    }
    assert_eq!(
        prepared(&plan, &schema).declines[0].1,
        Decline::NonStrictOrder
    );
}

#[test]
fn request_binding_rejects_a_second_rules_bag_sensitive_terminal_select() {
    let (first, _) = fixture();
    let mut second = first.clone();
    second.steps.truncate(3);
    second.emit.step = StepId(2); // Direct observation of the pair-multiplicity bag.
    let plans = [first, second];
    let request = crate::domain::operator::contract::types::RequestOperators::prepare(
        &plans,
        &[vec![], vec![]],
        &[],
        (std::sync::Arc::new(
            ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
    )
    .unwrap();
    assert_eq!(
        request.ordered_existence[0].shapes.len(),
        1,
        "local shape alone is insufficient"
    );
    let graph = crate::domain::file_program::service::compile_prepared(&plans, &request);
    use crate::domain::ordered_existence::types::ResolvedAlternative;
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
}
