//! Observation capture and diagnostic delivery are bound before admission.
use crate::boundary_fixture::fixtures::{corpus, demand, engine, prepared, source};
use crate::file_intents::fixtures::input;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileExecution;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::rows::types::{RowCount, Table};
use std::cell::Cell;

#[test]
fn diagnostic_delivery_is_bound_before_admission_and_cannot_switch_after_publication() {
    let engine = engine();
    let program = prepared(&engine);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let table = source();
    for diagnostic in [DiagnosticDelivery::Collected, DiagnosticDelivery::Forwarded] {
        let reads = Cell::new(0);
        let read = |relation| {
            assert_eq!(relation, Relation::Function);
            reads.set(reads.get() + 1);
            Ok(SourceRows::Computed(&table))
        };
        let intents = [FileRuleIntent::Run {
            diagnostic,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }];
        let mut symbols = Symbols::default();
        let mut file = request
            .open_file(input(&intents, &elements, &read), &mut symbols)
            .unwrap();
        match diagnostic {
            DiagnosticDelivery::Collected => assert!(matches!(
                file.emit_rows(RuleIndex(0), &mut symbols, &mut |_| panic!(
                    "wrong mode cannot deliver"
                )),
                Err(OperatingError::NotRun)
            )),
            DiagnosticDelivery::Forwarded => assert!(matches!(
                file.rule(RuleIndex(0), demand(), &mut symbols),
                Err(OperatingError::NotRun)
            )),
            DiagnosticDelivery::Disabled => unreachable!(),
        }
        assert_eq!(reads.get(), 0, "wrong mode cannot admit any provider");
        assert!(
            file.fixture_counts(RuleIndex(0))
                .unwrap()
                .fed
                .iter()
                .all(|count| count.0 == [0; 3])
        );
        let expected = Table {
            kinds: vec![CellKind::Node],
            count: RowCount(1),
            cells: vec![table.cells[0]],
        };
        match diagnostic {
            DiagnosticDelivery::Collected => {
                let RootOutcome::Complete(actual) = file
                    .rule(RuleIndex(0), demand(), &mut symbols)
                    .unwrap()
                    .emit
                else {
                    panic!("correct declared collected mode remains runnable")
                };
                assert_eq!(actual, expected);
            }
            DiagnosticDelivery::Forwarded => {
                let mut rows = vec![];
                file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
                    rows.push(row.to_vec());
                    Ok(())
                })
                .expect("correct declared forwarded mode remains runnable");
                assert_eq!(rows, [expected.cells]);
            }
            DiagnosticDelivery::Disabled => unreachable!(),
        }
        let admitted_reads = reads.get();
        assert!(admitted_reads > 0);
        match diagnostic {
            DiagnosticDelivery::Collected => assert!(matches!(
                file.emit_rows(RuleIndex(0), &mut symbols, &mut |_| panic!(
                    "publication cannot change mode"
                )),
                Err(OperatingError::NotRun)
            )),
            DiagnosticDelivery::Forwarded => assert!(matches!(
                file.rule(RuleIndex(0), demand(), &mut symbols),
                Err(OperatingError::NotRun)
            )),
            DiagnosticDelivery::Disabled => unreachable!(),
        }
        assert_eq!(
            reads.get(),
            admitted_reads,
            "mode rejection cannot reread a published provider"
        );
        file.finish().unwrap();
    }
    request.finish();
}
