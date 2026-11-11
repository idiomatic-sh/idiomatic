//! A selected join shares work, keeps counts and exports owned frontiers.
use super::super::super::fixtures::{complete, input};
use super::fixtures::{functions, selected_join};
use crate::boundary_fixture::fixtures::{authored_request, corpus, demand, engine};
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::ProgramFileSessionPort as _;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort as _;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase as _;
use ryo_execution_runtime::application::file_dag::program::file_session::types::CaptureContext;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::fixtures::{kinds, node_id, to_bits};
use ryo_relations::domain::relations::types::*;
use ryo_values::domain::rows::types::{RowCount, Table};
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::table_words::fixtures::decode_complete;
use std::cell::Cell;
#[test]
fn selected_join_shares_work_preserves_counts_and_exports_owned_frontiers() {
    for distinct in [false, true] {
        for stored in [false, true] {
            let engine = engine();
            let mut authored = authored_request(2);
            *crate::boundary_fixture::fixtures::plans(&mut authored) =
                vec![selected_join(distinct); 2];
            let program = engine.prepare(authored).unwrap();
            let corpus = corpus();
            let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
            let mut symbols = Symbols::default();
            let source = functions(&mut symbols);
            let columns: Vec<FactColumn> = (0..source.kinds.len())
                .map(|column| {
                    FactColumn(
                        source
                            .cells
                            .chunks(source.kinds.len())
                            .map(|row| to_bits(row[column]).0)
                            .collect(),
                    )
                })
                .collect();
            let reads = Cell::new(0);
            let read = |relation| {
                reads.set(reads.get() + 1);
                if relation == Relation::Function {
                    Ok(if stored {
                        SourceRows::Stored(ColumnsView {
                            kinds: kinds(Relation::Function),
                            columns: &columns,
                        })
                    } else {
                        SourceRows::Computed(&source)
                    })
                } else {
                    Err(OperatingError::NotRead)
                }
            };
            let elements = Elements::default();
            let intents = [FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Collected,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::DiagnosticAndObservations,
            }; 2];
            let mut file = request
                .open_file(
                    input(FileId(0), b"", &elements, &intents, &read),
                    &mut symbols,
                )
                .unwrap();
            let expected = Table {
                kinds: vec![
                    if distinct {
                        CellKind::File
                    } else {
                        CellKind::Node
                    },
                    CellKind::File,
                ],
                count: RowCount(8),
                cells: (0..8)
                    .flat_map(|row| {
                        [
                            if distinct {
                                Value::File(FileId(0))
                            } else {
                                Value::Node(node_id(FileId(0), Position(row * 8)))
                            },
                            Value::File(FileId(0)),
                        ]
                    })
                    .collect(),
            };
            let expected_frontier = Table {
                kinds: vec![CellKind::Node, CellKind::File],
                count: RowCount(8),
                cells: (0..8)
                    .flat_map(|row| {
                        [
                            Value::Node(node_id(FileId(0), Position(row * 8))),
                            Value::File(FileId(0)),
                        ]
                    })
                    .collect(),
            };
            let first = file.rule(RuleIndex(0), demand(), &mut symbols).unwrap();
            assert_eq!(complete(first), expected);
            let reads_after_run = reads.get();
            let mut escaped = Vec::new();
            for rule in [RuleIndex(0), RuleIndex(1)] {
                let before_delivery = reads.get();
                let outcome = file.rule(rule, demand(), &mut symbols).unwrap();
                assert_eq!(complete(outcome), expected);
                assert_eq!(
                    reads.get(),
                    before_delivery,
                    "publication replay does not borrow a provider"
                );
                let (mut arena, mut places, mut words) = (vec![], vec![], Words::default());
                let captures = file
                    .capture_frontiers(
                        rule,
                        Bucket(0),
                        &mut symbols,
                        CaptureContext {
                            worker: WorkerNumber(0),
                            arena: ryo_execution::inbound::use_cases::execution::interface::Arena(
                                &mut arena,
                            ),
                            places: &mut places,
                            words: &mut words,
                        },
                    )
                    .unwrap();
                assert_eq!(captures.len(), 1);
                assert_eq!(
                    reads.get(),
                    before_delivery,
                    "a captured selection is owned rows over either backing: capture borrows no provider"
                );
                escaped.extend(captures.into_iter().map(|capture| capture.outcome.unwrap()));
            }
            assert_eq!(
                reads.get(),
                reads_after_run,
                "no capture of either backing reads its provider again"
            );
            file.finish().unwrap();
            // The actual encoded publication must survive its file execution owner.
            for frontier in escaped {
                let decoded = decode_complete(&frontier.words, &mut Vec::new()).unwrap();
                assert_eq!(decoded, expected_frontier);
            }
            // A new generation cannot answer an unavailable provider from the previous file cache.
            let refused = |_| Err(OperatingError::NotRead);
            let mut next = request
                .open_file(
                    input(FileId(0), b"", &elements, &intents, &refused),
                    &mut symbols,
                )
                .unwrap();
            for rule in [RuleIndex(0), RuleIndex(1)] {
                let result = next.rule(rule, demand(), &mut symbols).unwrap();
                assert!(matches!(
                    result.emit,
                    RootOutcome::Failed(OperatingError::NotRead)
                ));
            }
            next.finish().unwrap();
            request.finish();
        }
    }
}
