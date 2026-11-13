//! Publication policy is enforced by fresh public delivery, never a pre-collected root.
use super::super::*;
use super::fixtures::{abc_file, function_rows, program, rows};
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;

#[test]
fn semantic_failure_discards_every_after_success_row() {
    for second in [(2, 1), (4, 7)] {
        for forwarded in [true, false] {
            let engine = engine();
            let prepared = program(&engine);
            let corpus = corpus();
            let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None.into());
            let table = rows(second);
            let elements = ryo_language::domain::elements::types::Elements::default();
            let read = function_rows(&table);
            let mut symbols = Symbols::default();
            let intents = [FileRuleIntent::Run {
                diagnostic: if forwarded {
                    DiagnosticDelivery::Forwarded
                } else {
                    DiagnosticDelivery::Collected
                },
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::Disabled,
            }];
            let mut file = request
                .open_file(abc_file(&intents, &elements, &read), &mut symbols)
                .expect("open actual file");
            if forwarded {
                let mut exposed = Vec::new();
                let result = file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
                    exposed.extend_from_slice(row);
                    Ok(())
                });
                assert!(matches!(result, Err(OperatingError::InvalidSpan)));
                assert!(
                    exposed.is_empty(),
                    "a later semantic refusal discards the entire batch"
                );
            } else {
                let outcome = file.rule(RuleIndex(0), demand(), &mut symbols).unwrap();
                assert!(matches!(
                    outcome.emit,
                    RootOutcome::Failed(OperatingError::InvalidSpan)
                ));
            }
            file.finish().expect("release failed invocation");
            request.finish();
        }
    }
}

#[test]
fn committed_rows_keep_order_and_delivery_refusal_remains_retryable() {
    for (refuse, diagnostic) in [
        (false, DiagnosticDelivery::Forwarded),
        (true, DiagnosticDelivery::Forwarded),
        (false, DiagnosticDelivery::Collected),
    ] {
        let engine = engine();
        let prepared = program(&engine);
        let corpus = corpus();
        let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None.into());
        let table = rows((1, 2));
        let elements = ryo_language::domain::elements::types::Elements::default();
        let read = function_rows(&table);
        let mut symbols = Symbols::default();
        let intents = [FileRuleIntent::Run {
            diagnostic,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }];
        let mut file = request
            .open_file(abc_file(&intents, &elements, &read), &mut symbols)
            .expect("open actual file");
        if diagnostic == DiagnosticDelivery::Collected {
            let RootOutcome::Complete(retained) = file
                .rule(RuleIndex(0), demand(), &mut symbols)
                .unwrap()
                .emit
            else {
                panic!("successful collected publication")
            };
            assert_eq!(retained, table);
            file.finish().expect("release collected invocation");
            request.finish();
            continue;
        }
        let mut exposed = Vec::new();
        let result = file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
            if refuse && !exposed.is_empty() {
                return Err(OperatingError::WrongBinding);
            }
            exposed.extend_from_slice(row);
            Ok(())
        });
        if refuse {
            assert!(matches!(result, Err(OperatingError::WrongBinding)));
            assert_eq!(exposed, table.cells[..table.kinds.len()]);
            exposed.clear();
            file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
                exposed.extend_from_slice(row);
                Ok(())
            })
            .expect("retry after delivery refusal");
        } else {
            result.expect("publish after successful semantics");
        }
        assert_eq!(exposed, table.cells);
        assert!(
            matches!(
                file.rule(RuleIndex(0), demand(), &mut symbols),
                Err(OperatingError::NotRun)
            ),
            "forwarded publication cannot be reopened in collected mode"
        );
        file.finish().expect("release successful invocation");
        request.finish();
    }
}
