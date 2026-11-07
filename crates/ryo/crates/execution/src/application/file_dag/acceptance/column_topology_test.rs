//! Request-wide physical replacement must respect every reader of canonical nodes.
use crate::domain::executable_program::column::ColumnCandidates;
use crate::domain::executable_program::service;
use crate::domain::operating::errors::OperatingError;
#[cfg(test)]
mod fixtures;
use fixtures::{column, plan, join_output_observation, ordinary_plan, graph, node, program, assert_one_route, column_count};
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::Case;
use ryo_values::cases::CaseName;
use ryo_values::cases::Expect;
use ryo_values::cases::Word;
use ryo_values::cases::run;
use ryo_values::domain::observations::types::ObservationColumn;
use ryo_values::domain::observations::types::ObservationColumnName;
use ryo_values::domain::observations::types::ObservationName;
use ryo_values::domain::observations::types::PlanObservation;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::plans::types::PlanStep;

fn exclusive_chain_lowers_once_and_duplicate_rule_shares_one_physical_owner() -> usize {
    let single = vec![ordinary_plan()];
    let (operators, request) = graph(&single);
    assert_eq!(ColumnCandidates::capture(&request).fences().count(), 4);
    let compiled = program(&single, &operators, request);
    assert_eq!(column_count(&compiled), 1);
    assert!(
        service::column(&compiled.regions()[compiled.rules()[0].emit.as_ref().unwrap().0].route)
            .is_some()
    );

    let duplicate = vec![ordinary_plan(), ordinary_plan()];
    let (operators, request) = graph(&duplicate);
    for step in 0..4 {
        assert_eq!(node(&request, 0, step), node(&request, 1, step));
    }
    let left_node = node(&request, 0, 0);
    assert_eq!(ColumnCandidates::capture(&request).fences().count(), 8);
    let (ordinary_operators, mut without_replacement) = graph(&duplicate);
    let shared_join = node(&without_replacement, 0, 1);
    without_replacement.derived_sources[shared_join.0] = true;
    let ordinary = program(&duplicate, &ordinary_operators, without_replacement);
    let compiled = program(&duplicate, &operators, request);
    assert_eq!(column_count(&compiled), 1);
    assert_eq!(compiled.rules()[0].emit, compiled.rules()[1].emit);
    let left = compiled.logical_regions()[left_node.0];
    let ordinary_left = ordinary.logical_regions()[left_node.0];
    assert_eq!(
        compiled.regions()[left.0].consumers,
        ordinary.regions()[ordinary_left.0].consumers + 1,
        "one physical alternative owns one extra source reader, never one per duplicate rule"
    );
    column_count(&compiled)
}

fn distinct_targets_with_shared_join_and_filter_keep_both_ordinary_in_either_rule_order() -> usize {
    let first = ordinary_plan();
    let second = plan(vec![column(0), column(1)]);
    for plans in [
        vec![first.clone(), second.clone()],
        vec![second.clone(), first.clone()],
    ] {
        let (operators, request) = graph(&plans);
        assert_eq!(node(&request, 0, 1), node(&request, 1, 1));
        assert_eq!(node(&request, 0, 2), node(&request, 1, 2));
        assert_ne!(node(&request, 0, 3), node(&request, 1, 3));
        let filter_node = node(&request, 0, 2);
        assert_eq!(ColumnCandidates::capture(&request).fences().count(), 8);
        let compiled = program(&plans, &operators, request);
        assert_eq!(column_count(&compiled), 0);
        assert_ne!(compiled.rules()[0].emit, compiled.rules()[1].emit);
        let filter = compiled.logical_regions()[filter_node.0];
        assert_eq!(compiled.regions()[filter.0].consumers, 2);
    }
    0
}

fn emit_frontier_named_and_derived_ancestor_boundaries_prevent_replacement() -> usize {
    let mut emit = ordinary_plan();
    emit.emit.step = StepId(1);
    emit.frontier.push(PlanInput::Step(StepId(3)));

    let mut frontier = ordinary_plan();
    frontier.frontier.push(PlanInput::Step(StepId(2)));

    let mut named = ordinary_plan();
    named.observations.push(join_output_observation());

    for (observed, boundary_step) in [(emit, 1), (frontier, 2), (named, 1)] {
        let plans = vec![observed];
        let (operators, request) = graph(&plans);
        let boundary = node(&request, 0, boundary_step);
        let terminal = node(&request, 0, 3);
        let compiled = program(&plans, &operators, request);
        assert_eq!(column_count(&compiled), 0);
        assert_ne!(
            compiled.logical_regions()[boundary.0],
            compiled.logical_regions()[terminal.0],
            "an externally observed ancestor keeps its own physical publication"
        );
    }

    let plans = vec![ordinary_plan()];
    let (operators, mut request) = graph(&plans);
    let filter = node(&request, 0, 2);
    assert_eq!(ColumnCandidates::capture(&request).fences().count(), 4);
    request.derived_sources[filter.0] = true;
    assert_eq!(column_count(&program(&plans, &operators, request)), 0);
    0
}

/// A native-eligible Join that a named observation publishes and a Project reads is read by
/// rows, so neither reader may be handed a native positions publication.
fn native_join_read_by_rows_keeps_no_native_positions() -> usize {
    let read = Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: Vec::new(),
        steps: vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: PlanInput::Relation(Relation::SyntaxTree),
                    comparisons: Vec::new(),
                    kept: Vec::new(),
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Join {
                    left: PlanInput::Step(StepId(0)),
                    right: PlanInput::Relation(Relation::SyntaxChild),
                    left_on: vec![column(0)],
                    right_on: vec![column(0)],
                    output: vec![
                        JoinColumn {
                            side: Side::Left,
                            column: column(0),
                        },
                        JoinColumn {
                            side: Side::Right,
                            column: column(1),
                        },
                    ],
                    filter: Vec::new(),
                    view: Vec::new(),
                    reach: None,
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Project {
                    from: PlanInput::Step(StepId(1)),
                    columns: vec![column(0), column(1)],
                    distinct: RepeatedRows::Dropped,
                },
            },
        ],
        comparison_origins: Vec::new(),
        emit: Emit {
            step: StepId(2),
            subject: column(0),
            key: vec![column(0)],
            headings: Vec::new(),
        },
        frontier: Vec::new(),
    };
    let mut observed = read.clone();
    observed.observations.push(PlanObservation {
        name: ObservationName("parent-child".into()),
        source: PlanInput::Step(StepId(1)),
        columns: ["parent", "child"]
            .map(|name| ObservationColumn {
                name: ObservationColumnName(name.into()),
                kind: CellKind::Node,
            })
            .to_vec(),
        scope: StepScope::PerFile,
    });
    let native = |plan: &Plan| {
        let plans = vec![plan.clone()];
        let (operators, request) = graph(&plans);
        let join = node(&request, 0, 1);
        let compiled = program(&plans, &operators, request);
        assert_one_route(&compiled);
        service::native_positions(&compiled.regions()[compiled.logical_regions()[join.0].0].route)
            .is_some()
    };
    assert!(
        native(&read),
        "the Join is native-eligible when nothing reads its rows"
    );
    assert!(
        !native(&observed),
        "an observed Join is read by rows and keeps no native positions"
    );
    0
}

#[derive(Clone, Copy)]
enum Scenario {
    ExclusiveAndDuplicate,
    SharedAncestors,
    ObservedAncestors,
    NativeJoinObserved,
}

type Input = Scenario;
type Value = usize;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("exclusive_chain_is_selected_once_even_for_duplicate_rules"),
        word: Word::Succeeds,
        input: || Scenario::ExclusiveAndDuplicate,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("shared_join_and_filter_keep_both_targets_ordinary_in_either_order"),
        word: Word::Succeeds,
        input: || Scenario::SharedAncestors,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("emit_frontier_named_and_derived_roots_prevent_replacement"),
        word: Word::Succeeds,
        input: || Scenario::ObservedAncestors,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("native_join_observed_by_a_named_output_keeps_no_native_positions"),
        word: Word::Succeeds,
        input: || Scenario::NativeJoinObserved,
        expect: Expect::Value(0),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(match scenario {
        Scenario::ExclusiveAndDuplicate => {
            exclusive_chain_lowers_once_and_duplicate_rule_shares_one_physical_owner()
        }
        Scenario::SharedAncestors => {
            distinct_targets_with_shared_join_and_filter_keep_both_ordinary_in_either_rule_order()
        }
        Scenario::ObservedAncestors => {
            emit_frontier_named_and_derived_ancestor_boundaries_prevent_replacement()
        }
        Scenario::NativeJoinObserved => native_join_read_by_rows_keeps_no_native_positions(),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
