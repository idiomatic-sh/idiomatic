//! A selected join keeps counts and exports owned frontiers.
use super::fixtures::{functions, selected_join};
use crate::boundary_fixture::fixtures::{
    FileRun, authored_request, corpus, engine, ran, root, run_file,
};
use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort as _;
use ryo_execution::inbound::use_cases::execution::interface::RootOutcome;
use ryo_execution::inbound::use_cases::execution::interface::{
    EmittedRoot, FileEnd, FileStop, RuleAfter,
};
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase as _;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::fixtures::node_id;
use ryo_relations::domain::relations::types::*;
use ryo_values::domain::rows::types::{RowCount, Table};
use ryo_values::domain::table_words::fixtures::decode_complete;
#[test]
fn selected_join_shares_work_preserves_counts_and_exports_owned_frontiers() {
    for distinct in [false, true] {
        {
            let engine = engine();
            let mut authored = authored_request(2);
            *crate::boundary_fixture::fixtures::plans(&mut authored) =
                vec![selected_join(distinct); 2];
            let program = engine.prepare(authored).unwrap();
            let corpus = corpus();
            let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
            let mut symbols = Symbols::default();
            let source = functions(&mut symbols);
            // A fact relation reaches the run as the worker's stored columns, the one backing `run_file` reads.
            let mut read = TableFacts::new(&[(Relation::Function, &source)]);
            let elements = Elements::default();
            let intents = [FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Collected,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::DiagnosticAndObservations,
            }; 2];
            let mut rules = run_file(
                &request,
                FileRun {
                    file: FileId(0),
                    source: b"",
                    ..FileRun::of(&intents, &elements)
                },
                &mut symbols,
                &mut read,
            );
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
            let mut escaped = Vec::new();
            for rule in [0, 1] {
                let ran = ran(&mut rules, rule);
                let EmittedRoot::Delivered(RootOutcome::Complete(table)) = ran.emit else {
                    panic!("rule {rule} delivers its root complete")
                };
                assert_eq!(table, expected);
                let RuleAfter::Ran { captures } = ran.after else {
                    panic!("rule {rule} captures its frontier")
                };
                assert_eq!(captures.len(), 1);
                escaped.extend(captures.into_iter().map(|capture| capture.outcome.unwrap()));
            }
            assert!(read.reads() > 0);
            assert!(matches!(rules.end, FileEnd::Finished(_)));
            // The actual encoded publication must survive its file execution owner.
            for frontier in escaped {
                let decoded = decode_complete(&frontier.words, &mut Vec::new()).unwrap();
                assert_eq!(decoded, expected_frontier);
            }
            // A new run cannot answer an unavailable provider from the previous file's cache.
            let mut refused = TableFacts::new(&[]);
            let mut next = run_file(
                &request,
                FileRun {
                    source: b"",
                    ..FileRun::of(&intents, &elements)
                },
                &mut symbols,
                &mut refused,
            );
            assert!(matches!(root(ran(&mut next, 0)), Err(FileStop::Facts(_))));
            request.finish();
        }
    }
}
