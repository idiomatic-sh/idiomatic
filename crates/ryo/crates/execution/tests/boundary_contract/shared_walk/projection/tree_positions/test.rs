//! Shared tree positions keep native join projection refusal, and pure projections share one scan.
use super::super::super::fixtures::*;
use crate::boundary_fixture::fixtures::{authored_request, corpus, ran, root};
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileEnd;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::RootOutcome;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_relations::domain::relations::fixtures::node_id;
use ryo_relations::domain::relations::types::{FileId, Position, Relation, Value};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, StepId, Test};
use ryo_values::domain::plans::types::{Operation, PlanStep};
#[test]
fn shared_tree_positions_keep_native_join_projection_refusal_and_replay() {
    use ryo_execution_runtime::outbound::operators::fixtures::projection_tree_positions::ObservedJoin;
    use ryo_values::domain::plan_parts::types::{JoinColumn, Side};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    for relation in [Relation::SyntaxTree, Relation::SyntaxChild] {
        for (empty, refuse) in [(false, false), (true, false), (false, true)] {
            let semantics = Arc::new(ObservedJoin {
                calls: AtomicUsize::new(0),
                refuse,
            });
            let engine = PrepareUseCaseImpl::new(
                semantics.clone(),
                Arc::new(ryo_execution::inbound::use_cases::execution::fixtures::derived()).into(),
                Arc::new(ryo_execution_runtime::outbound::digest::factory::blake3()),
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
            crate::boundary_fixture::fixtures::plans(&mut authored)[0].steps = vec![
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
            crate::boundary_fixture::fixtures::plans(&mut authored)[0]
                .emit
                .step = StepId(1);
            let program = engine.prepare(authored).unwrap();
            let corpus = corpus();
            let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
            let (elements, mut symbols) = tree(FileId(0), b"abc");
            let intents = [FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Collected,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::DiagnosticAndObservations,
            }];
            let mut rules = run(
                &request,
                (FileId(0), b"abc"),
                &elements,
                &intents,
                &mut symbols,
            );
            if refuse {
                assert!(matches!(
                    root(ran(&mut rules, 0)),
                    Ok(RootOutcome::Failed(OperatingError::ShapeMismatch))
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
                assert_eq!(complete(&mut rules, 0).cells, expected);
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
            request.finish();
        }
    }
}
#[test]
fn direct_pure_projections_share_source_scan_including_zero_columns_and_repeats() {
    use ryo_values::domain::plan_parts::types::RepeatedRows;
    let (engine, semantics) = engine();
    let mut authored = authored_request(3);
    for (plan, columns) in crate::boundary_fixture::fixtures::plans(&mut authored)
        .iter_mut()
        .zip([
            vec![ColumnIndex(2), ColumnIndex(2)],
            vec![],
            vec![ColumnIndex(1)],
        ])
    {
        plan.steps[0].operation = Operation::Project {
            from: Input::Relation(Relation::SyntaxTree),
            columns,
            distinct: RepeatedRows::Kept,
        };
    }
    let program = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::DiagnosticAndObservations,
    }; 3];
    let mut rules = run(
        &request,
        (FileId(0), b"abc"),
        &elements,
        &intents,
        &mut symbols,
    );
    for (rule, width) in [(0, 2), (1, 0), (2, 1)] {
        let actual = complete(&mut rules, rule);
        assert_eq!(actual.count.0, 4);
        assert_eq!(actual.kinds.len(), width);
        assert_eq!(actual.cells.len(), width * 4);
        if width == 2 {
            for row in actual.cells.chunks(2) {
                assert_eq!(row[0], row[1]);
            }
        }
    }
    assert_eq!(semantics.selects(), 0);
    assert!(matches!(rules.end, FileEnd::Finished(_)));
    request.finish();
}
