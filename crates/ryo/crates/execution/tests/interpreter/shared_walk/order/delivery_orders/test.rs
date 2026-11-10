//! Frontier and callback delivery share one atomic failure and one dependency contract in both orders.
use crate::boundary_fixture::{authored_request, corpus};
use crate::interpreter::shared_walk::fixtures::counts;
use crate::shared_walk::fixtures::*;
use crate::shared_walk::no_catalog;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution_runtime::application::file_dag::program::file_session::interface::FileExecution;
use ryo_execution_runtime::application::file_dag::program::file_session::types::CaptureContext;
use ryo_relations::domain::relations::types::{FileId, Relation};
use ryo_values::domain::plan_parts::types::{Input, StepId};
use ryo_values::domain::plans::types::Operation;
#[test]
fn frontier_and_callback_share_the_same_atomic_semantic_failure() {
    use ryo_relations::domain::relations::types::{Words, WorkerNumber};
    for capture_first in [true, false] {
        let (engine, _) = engine();
        let mut authored = authored_request(1);
        crate::boundary_fixture::plans(&mut authored)[0].steps[0].operation = Operation::Select {
            from: Input::Relation(Relation::SyntaxTree),
            comparisons: selection(Selector::DynamicRegex),
            kept: vec![],
        };
        crate::boundary_fixture::plans(&mut authored)[0].frontier = vec![Input::Step(StepId(0))];
        let program = engine.prepare(authored).unwrap();
        let corpus = corpus();
        let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
        let (elements, mut symbols) = tree(FileId(0), b"ab(");
        let intents = [FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Forwarded,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        }];
        let mut file = request
            .open_file(
                input(FileId(0), b"ab(", &elements, &intents, &no_catalog),
                &mut symbols,
            )
            .unwrap();
        for capture_now in [capture_first, !capture_first] {
            if capture_now {
                let (mut arena, mut places, mut words) = (vec![], vec![], Words::default());
                let captured = file
                    .capture_frontiers(
                        RuleIndex(0),
                        ryo_values::domain::stored_facts::types::Bucket(0),
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
                assert_eq!(captured.len(), 1);
                assert!(
                    matches!(&captured[0].outcome, Err(error) if *error == regex_error("(")),
                    "a prefix followed by failure is never a complete frontier"
                );
            } else {
                let mut delivered = vec![];
                let result = file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
                    delivered.extend_from_slice(row);
                    Ok(())
                });
                assert!(matches!(result, Err(error) if error == regex_error("(")));
                assert!(
                    delivered.is_empty(),
                    "failed evaluation publishes no prefix"
                );
            }
        }
        counts(&file, 1, 4);
        assert_eq!(file.fixture_walk_counts().predicate_rows.0, 2);
        file.finish().unwrap();
        request.finish();
    }
}
