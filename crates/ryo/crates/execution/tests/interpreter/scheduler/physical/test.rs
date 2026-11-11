//! Hosted ordered dependencies use the same scheduler for both delivery modes.
use crate::boundary_fixture::fixtures::{corpus, demand, engine};
use crate::scheduler::physical::fixtures::ordered;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest as _;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileExecution as _;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceLanguage;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_relations::domain::relations::types::{CellKind, FileId, Integer, Value};
use ryo_values::domain::rows::types::{RowCount, Table};

#[test]
fn ordered_hosted_inputs_are_scheduled_once_before_collected_or_forwarded_delivery() {
    for diagnostic in [DiagnosticDelivery::Collected, DiagnosticDelivery::Forwarded] {
        let engine = engine();
        let program = engine.prepare(ordered()).unwrap();
        let corpus = corpus();
        let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
        let elements = Elements::default();
        let read = |_| panic!("hosted inputs are immutable request data, never provider relations");
        let mut symbols = Symbols::default();
        let intents = [FileRuleIntent::Run {
            diagnostic,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }; 2];
        let mut file = request
            .open_file(
                FileInput {
                    language: SourceLanguage::Recognized,
                    resident_stability: ResidentStability::Stable,
                    file: FileId(0),
                    evaluation: &[],
                    publication: &[],
                    intents: &intents,
                    source: ryo_values::domain::files::types::SourceView(b"abc"),
                    elements: &elements,
                    read: &read,
                    semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                    kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
                },
                &mut symbols,
            )
            .unwrap();
        assert_eq!(file.fixture_readiness_counts().evaluations.0, 0);
        let mut first_work = None;
        for index in [1, 0, 1] {
            let actual = match diagnostic {
                DiagnosticDelivery::Collected => {
                    let RootOutcome::Complete(table) = file
                        .rule(RuleIndex(index), demand(), &mut symbols)
                        .unwrap()
                        .emit
                    else {
                        panic!("ordered collected result")
                    };
                    table
                }
                DiagnosticDelivery::Forwarded => {
                    let mut cells = vec![];
                    let mut rows = 0;
                    file.emit_rows(RuleIndex(index), &mut symbols, &mut |row| {
                        rows += 1;
                        cells.extend_from_slice(row);
                        Ok(())
                    })
                    .unwrap();
                    Table {
                        kinds: vec![CellKind::Number; 2],
                        count: RowCount(rows),
                        cells,
                    }
                }
                DiagnosticDelivery::Disabled => unreachable!(),
            };
            assert_eq!(
                actual,
                Table {
                    kinds: vec![CellKind::Number; 2],
                    count: RowCount(2),
                    cells: [1, 2, 5, 6]
                        .map(|value| Value::Number(Integer(value)))
                        .to_vec()
                }
            );
            let work = file.fixture_readiness_counts();
            assert!(
                work.evaluations.0 > 0,
                "ordered route must enter the canonical scheduler"
            );
            if let Some(first) = first_work {
                assert_eq!(
                    work, first,
                    "logical aliases cannot reschedule physical producers"
                );
            } else {
                first_work = Some(work);
            }
        }
        file.finish().unwrap();
        request.finish();
    }
}
