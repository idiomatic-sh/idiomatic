//! Publication policy is enforced by fresh public delivery, never a pre-collected root.
use super::*;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_values::domain::plan_parts::types::LiteralText;

pub(crate) fn program(engine: &PrepareUseCaseImpl) -> PreparedExecutionProgram {
    let mut input = authored_request(1);
    crate::boundary_fixture::plans(&mut input)[0].steps = vec![PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::FitsRendered {
            from: Input::Relation(Relation::Function),
            texts: vec![],
            literals: vec![LiteralText("x".into())],
            span: ColumnIndex(3),
            max_line: ryo_values::domain::query_operators::types::MaxLine(80),
            tab_width: ryo_values::domain::query_operators::types::TabWidth(4),
        },
    }];
    crate::boundary_fixture::plans(&mut input)[0].emit.step = StepId(0);
    engine.prepare(input).expect("actual FitsRendered program")
}

pub(crate) fn rows(second: (u32, u32)) -> Table {
    let mut table = source();
    let mut first = table.cells.clone();
    first[3] = Value::Span(Span {
        start: Offset(0),
        end: Offset(1),
    });
    let mut next = first.clone();
    next[0] = Value::Node(node_id(FileId(0), Position(3)));
    next[3] = Value::Span(Span {
        start: Offset(second.0),
        end: Offset(second.1),
    });
    table.cells = [first, next].concat();
    table.count = RowCount(2);
    table
}

#[test]
fn semantic_failure_discards_every_after_success_row() {
    for second in [(2, 1), (4, 7)] {
        for forwarded in [true, false] {
            let engine = engine();
            let prepared = program(&engine);
            let corpus = corpus();
            let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None);
            let table = rows(second);
            let elements = ryo_language::domain::elements::types::Elements::default();
            let read = |relation| {
                assert_eq!(relation, Relation::Function);
                Ok(SourceRows::Computed(&table))
            };
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
        let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None);
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
                    semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                    kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
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

/// A store of its own, through the store port's fixture, over the store's derived semantics.
pub(crate) fn store() -> impl ryo_execution_store::application::file_dag::relations::interface::RelationStore {
    use ryo_execution_store::application::file_dag::relations::interface::RelationStore as _;
    ryo_execution_store::application::file_dag::relations::RelationOwner::fixture(
        ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Eligible,
        Arc::new(ryo_execution_store::outbound::derived::factory::semantics()),
    )
}

#[cfg(test)]
mod axis;

mod frames;
mod store;
