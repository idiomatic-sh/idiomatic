//! Publication policy is enforced by fresh public delivery, never a pre-collected root.
use super::super::*;
use super::fixtures::{function_rows, program, rows};

#[test]
fn semantic_failure_discards_every_after_success_row() {
    for second in [(2, 1), (4, 7)] {
        let engine = engine();
        let prepared = program(&engine);
        let corpus = corpus();
        let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None.into());
        let table = rows(second);
        let elements = ryo_language::domain::elements::types::Elements::default();
        let facts = function_rows(&table);
        let mut symbols = Symbols::default();
        let intents = [FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        }];
        let mut rules = run_file(
            &request,
            FileRun::of(&intents, &elements),
            &mut symbols,
            &facts,
        );
        assert!(
            matches!(
                root(ran(&mut rules, 0)),
                Ok(RootOutcome::Failed(OperatingError::InvalidSpan))
            ),
            "a later semantic refusal discards the entire batch"
        );
        request.finish();
    }
}

#[test]
fn committed_rows_keep_order_and_delivery_refusal_remains_retryable() {
    for diagnostic in [DiagnosticDelivery::Collected] {
        let engine = engine();
        let prepared = program(&engine);
        let corpus = corpus();
        let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None.into());
        let table = rows((1, 2));
        let elements = ryo_language::domain::elements::types::Elements::default();
        let facts = function_rows(&table);
        let mut symbols = Symbols::default();
        let intents = [FileRuleIntent::Run {
            diagnostic,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        }];
        let mut rules = run_file(
            &request,
            FileRun::of(&intents, &elements),
            &mut symbols,
            &facts,
        );
        let Ok(RootOutcome::Complete(retained)) = root(ran(&mut rules, 0)) else {
            panic!("successful {diagnostic:?} publication")
        };
        assert_eq!(retained, table, "the committed rows keep their order");
        assert!(matches!(rules.end, FileEnd::Finished(_)));
        request.finish();
    }
}
