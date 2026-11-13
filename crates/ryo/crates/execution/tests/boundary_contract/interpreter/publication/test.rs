//! Publication policy is enforced by fresh public delivery, never a pre-collected root.
use crate::boundary_fixture::fixtures::{corpus, demand, engine};
use crate::publication::fixtures::{program, rows};
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileInput;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::ProgramFileSessionPort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::RootOutcome;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::interface::SourceLanguage;
use ryo_execution::inbound::use_cases::execution::interface::SourceRows;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{FileId, Relation};

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
        let read = |relation| {
            assert_eq!(relation, Relation::Function);
            Ok(SourceRows::Computed(&table))
        };
        let mut symbols = Symbols::default();
        let intents = [FileRuleIntent::Run {
            diagnostic,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }];
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
                    semantic: ryo_execution::inbound::use_cases::execution::interface::SemanticSupport::Absent,
                    kinds: ryo_execution::inbound::use_cases::execution::interface::KindSupport::Absent,
                },
                &mut symbols,
            )
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
            let evaluated = file
                .fixture_counts(RuleIndex(0))
                .unwrap()
                .fed
                .iter()
                .map(|counts| counts.0)
                .collect::<Vec<_>>();
            exposed.clear();
            file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
                exposed.extend_from_slice(row);
                Ok(())
            })
            .expect("retry after delivery refusal");
            assert_eq!(
                file.fixture_counts(RuleIndex(0))
                    .unwrap()
                    .fed
                    .iter()
                    .map(|counts| counts.0)
                    .collect::<Vec<_>>(),
                evaluated,
                "retrying a sink must not repeat semantic execution"
            );
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
