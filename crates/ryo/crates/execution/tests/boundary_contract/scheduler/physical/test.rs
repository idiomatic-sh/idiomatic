//! Hosted ordered dependencies use the same scheduler for both delivery modes.
use super::fixtures::ordered;
use crate::boundary_fixture::fixtures::{FileRun, corpus, engine, ran, root, run_file};
use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileEnd;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort as _;
use ryo_execution::inbound::use_cases::execution::interface::RootOutcome;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_relations::domain::relations::types::{CellKind, Integer, Value};
use ryo_values::domain::rows::types::{RowCount, Table};

#[test]
fn ordered_hosted_inputs_are_scheduled_once_before_collected_or_forwarded_delivery() {
    for diagnostic in [DiagnosticDelivery::Collected] {
        let engine = engine();
        let program = engine.prepare(ordered()).unwrap();
        let corpus = corpus();
        let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
        let elements = Elements::default();
        let mut facts = TableFacts::new(&[]);
        let mut symbols = Symbols::default();
        let intents = [FileRuleIntent::Run {
            diagnostic,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        }; 2];
        let mut rules = run_file(
            &request,
            FileRun::of(&intents, &elements),
            &mut symbols,
            &mut facts,
        );
        for index in [1, 0] {
            let Ok(RootOutcome::Complete(actual)) = root(ran(&mut rules, index)) else {
                panic!("ordered {diagnostic:?} result")
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
        }
        assert!(matches!(rules.end, FileEnd::Finished(_)));
        assert_eq!(
            facts.reads(),
            0,
            "hosted inputs are immutable request data, never provider relations"
        );
        request.finish();
    }
}
