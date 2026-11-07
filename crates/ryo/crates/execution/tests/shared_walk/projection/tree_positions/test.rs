//! Shared tree positions keep native join projection refusal and replay, and pure projections share one scan.
use super::super::super::fixtures::*;
use super::super::super::no_catalog;
use crate::boundary_fixture::{authored_request, corpus, demand};
use ryo_execution::inbound::use_cases::execution::interface::{
    ExecutionRequest, ExecutionUseCase, FileExecution,
};
use ryo_execution::inbound::use_cases::execution::types::*;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::{FileId, Position, Relation, Value};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, StepId, Test};
use ryo_values::domain::plans::types::{Operation, PlanStep};
#[test]
fn shared_tree_positions_keep_native_join_projection_refusal_and_replay() {
    use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
    use ryo_operators::domain::equality_join::types::JoinInput;
    use ryo_operators::domain::contract::types::OperatorAnswer;
    use ryo_operators::domain::contract::types::OperatorCall;
    use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
    use ryo_operators::domain::contract::types::PortableOperatorFault;
    use ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl;
    use ryo_values::domain::plan_parts::types::{JoinColumn, Side};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    #[derive(Debug)]
    struct ObservedJoin {
        calls: AtomicUsize,
        refuse: bool,
    }
    impl OperatorsUseCase for ObservedJoin {
        fn operators(
            &self,
            call: OperatorCall<'_>,
        ) -> Result<OperatorAnswer, PortableOperatorFault> {
            if matches!(
                &call,
                OperatorCall::EqualityJoin {
                    input: JoinInput::Positions { .. }
                }
            ) {
                self.calls.fetch_add(1, Ordering::Relaxed);
                if self.refuse {
                    return Err(PortableOperatorFault::ShapeMismatch);
                }
            }
            OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())).operators(call)
        }
    }
    for relation in [Relation::SyntaxTree, Relation::SyntaxChild] {
        for (empty, refuse) in [(false, false), (true, false), (false, true)] {
            let semantics = Arc::new(ObservedJoin {
                calls: AtomicUsize::new(0),
                refuse,
            });
            let engine = ExecutionUseCaseImpl::new(
                semantics.clone(),
                Arc::new(
                    ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
                ),
            );
            let mut authored = authored_request(1);
            let comparisons = if empty {
                // Comparing one node cell with itself is a total false predicate.
                vec![compared(
                    Test::NotEqual,
                    0,
                    ryo_values::domain::plan_parts::types::Operand::Column(ColumnIndex(0)),
                )]
            } else {
                vec![]
            };
            let column = if relation == Relation::SyntaxTree {
                0
            } else {
                1
            };
            authored.plans[0].steps = vec![
                PlanStep {
                    scope: StepScope::PerFile,
                    operation: Operation::Select {
                        from: Input::Relation(relation),
                        comparisons,
                        kept: vec![ColumnIndex(column), ColumnIndex(column)],
                    },
                },
                PlanStep {
                    scope: StepScope::PerFile,
                    operation: Operation::Join {
                        left: Input::Step(StepId(0)),
                        right: Input::Step(StepId(0)),
                        left_on: vec![ColumnIndex(0)],
                        right_on: vec![ColumnIndex(0)],
                        output: vec![
                            JoinColumn {
                                side: Side::Left,
                                column: ColumnIndex(0),
                            },
                            JoinColumn {
                                side: Side::Right,
                                column: ColumnIndex(1),
                            },
                        ],
                        filter: vec![],
                        view: vec![],
                        reach: None,
                    },
                },
            ];
            authored.plans[0].emit.step = StepId(1);
            let program = engine.prepare(authored).unwrap();
            let corpus = corpus();
            let request = engine.begin(&program, &corpus);
            let (elements, mut symbols) = tree(FileId(0), b"abc");
            let intents = [FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Collected,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::Disabled,
            }];
            let mut file = request
                .open_file(
                    input(FileId(0), b"abc", &elements, &intents, &no_catalog),
                    &mut symbols,
                )
                .unwrap();
            let first = file.rule(RuleIndex(0), demand(), &mut symbols).unwrap();
            if refuse {
                assert!(matches!(
                    first.emit,
                    RootOutcome::Failed(OperatingError::ShapeMismatch)
                ));
            } else {
                let start = u32::from(relation == Relation::SyntaxChild);
                let expected = if empty {
                    vec![]
                } else {
                    (start..4)
                        .flat_map(|position| {
                            [Value::Node(node_id(FileId(0), Position(position))); 2]
                        })
                        .collect()
                };
                assert_eq!(complete(first).cells, expected);
            }
            // The native join capability is invoked once per selected left row,
            // including a row with no RHS match, and stops at its first refusal.
            let expected_calls = if empty {
                0
            } else if refuse {
                1
            } else if relation == Relation::SyntaxTree {
                4
            } else {
                3
            };
            assert_eq!(semantics.calls.load(Ordering::Relaxed), expected_calls);
            let walks = file.walk_counts();
            assert_eq!(walks.passes.0, 1);
            assert_eq!(
                walks.source_rows.0,
                if relation == Relation::SyntaxTree {
                    4
                } else {
                    3
                }
            );
            let replay = file.rule(RuleIndex(0), demand(), &mut symbols).unwrap();
            if refuse {
                assert!(matches!(
                    replay.emit,
                    RootOutcome::Failed(OperatingError::ShapeMismatch)
                ));
            } else {
                assert_eq!(
                    complete(replay).count.0,
                    if empty {
                        0
                    } else if relation == Relation::SyntaxTree {
                        4
                    } else {
                        3
                    }
                );
            }
            assert_eq!(file.walk_counts(), walks);
            assert_eq!(semantics.calls.load(Ordering::Relaxed), expected_calls);
            file.finish().unwrap();
            request.finish();
        }
    }
}
#[test]
fn direct_pure_projections_share_source_scan_including_zero_columns_and_repeats() {
    use ryo_values::domain::plan_parts::types::RepeatedRows;
    let (engine, semantics) = engine();
    let mut authored = authored_request(3);
    for (plan, columns) in authored.plans.iter_mut().zip([
        vec![ColumnIndex(2), ColumnIndex(2)],
        vec![],
        vec![ColumnIndex(1)],
    ]) {
        plan.steps[0].operation = Operation::Project {
            from: Input::Relation(Relation::SyntaxTree),
            columns,
            distinct: RepeatedRows::Kept,
        };
    }
    let program = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = engine.begin(&program, &corpus);
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    }; 3];
    let mut file = request
        .open_file(
            input(FileId(0), b"abc", &elements, &intents, &no_catalog),
            &mut symbols,
        )
        .unwrap();
    for (rule, width) in [(RuleIndex(0), 2), (RuleIndex(1), 0), (RuleIndex(2), 1)] {
        let actual = complete(file.rule(rule, demand(), &mut symbols).unwrap());
        assert_eq!(actual.count.0, 4);
        assert_eq!(actual.kinds.len(), width);
        assert_eq!(actual.cells.len(), width * 4);
        if width == 2 {
            for row in actual.cells.chunks(2) {
                assert_eq!(row[0], row[1]);
            }
        }
    }
    let actual = file.walk_counts();
    assert_eq!(actual.passes.0, 1);
    assert_eq!(actual.source_rows.0, 4);
    assert_eq!(actual.scratch_classes.0, 1);
    assert_eq!(actual.result_slots.0, 1);
    assert_eq!(actual.predicate_tests.0, 0);
    assert_eq!(semantics.selects(), 0);
    file.finish().unwrap();
    request.finish();
}
